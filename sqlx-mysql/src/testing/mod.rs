use futures_core::future::BoxFuture;
use once_cell::sync::OnceCell;
use sqlx_core::connection::Connection;
use sqlx_core::query_scalar::query_scalar;
use std::ops::Deref;
use std::str::FromStr;
use std::time::Duration;

use crate::error::Error;
use crate::executor::Executor;
use crate::pool::{Pool, PoolOptions};
use crate::query::query;
use crate::{MySql, MySqlConnectOptions, MySqlConnection};
pub(crate) use sqlx_core::testing::*;

// Using a blocking `OnceCell` here because the critical sections are short.
static MASTER_POOL: OnceCell<Pool<MySql>> = OnceCell::new();
// Automatically delete any databases created before the start of the test binary.

impl TestSupport for MySql {
    fn test_context(args: &TestArgs) -> BoxFuture<'_, Result<TestContext<Self>, Error>> {
        Box::pin(async move { test_context(args).await })
    }

    fn cleanup_test(db_name: &str) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            let mut conn = MASTER_POOL
                .get()
                .expect("cleanup_test() invoked outside `#[sqlx::test]")
                .acquire()
                .await?;

            do_cleanup(&mut conn, db_name).await
        })
    }

    fn cleanup_test_dbs() -> BoxFuture<'static, Result<Option<usize>, Error>> {
        Box::pin(async move {
            let url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");

            let mut conn = MySqlConnection::connect(&url).await?;

            let delete_db_names: Vec<String> = query_scalar(indoc::indoc!(
                r#"
                SELECT db_name AS "db_name: _"
                FROM _sqlx_test.databases
                "#
            ))
            .fetch_all(&mut conn)
            .await?;

            if delete_db_names.is_empty() {
                return Ok(None);
            }

            let mut deleted_db_names = Vec::with_capacity(delete_db_names.len());

            for db_name in delete_db_names {
                match conn
                    .execute(format!("DROP DATABASE IF EXISTS {db_name};").as_str())
                    .await
                {
                    Ok(_deleted) => {
                        deleted_db_names.push(db_name);
                    }
                    // Assume a database error just means the DB is still in use.
                    Err(Error::Database(dbe)) => {
                        eprintln!("could not clean test database {db_name:?}: {dbe}")
                    }
                    // Bubble up other errors
                    Err(e) => return Err(e),
                }
            }

            let placeholders_str = if !deleted_db_names.is_empty() {
                deleted_db_names
                    .iter()
                    .map(|_| "?")
                    .fold(String::new(), |mut acc, placeholder| {
                        if !acc.is_empty() {
                            acc.push(',');
                        }
                        acc.push_str(placeholder);
                        acc
                    })
            } else {
                // MySQL does not consider the "in ()" syntax correct, so we add NULL
                "NULL".to_string()
            };

            let query_str = indoc::formatdoc!(
                r#"
                DELETE
                FROM _sqlx_test.databases
                WHERE db_name in ({placeholders_str});
                "#
            );
            let mut query = query(query_str.as_str());
            let deleted_db_names_count = deleted_db_names.len();

            for deleted_db_name in deleted_db_names {
                query = query.bind(deleted_db_name);
            }

            query.execute(&mut conn).await?;

            let _ = conn.close().await;
            Ok(Some(deleted_db_names_count))
        })
    }

    fn snapshot(
        _conn: &mut Self::Connection,
    ) -> BoxFuture<'_, Result<FixtureSnapshot<Self>, Error>> {
        // TODO: I want to get the testing feature out the door so this will have to wait,
        // but I'm keeping the code around for now because I plan to come back to it.
        todo!()
    }
}

async fn test_context(args: &TestArgs) -> Result<TestContext<MySql>, Error> {
    let url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let master_opts = MySqlConnectOptions::from_str(&url).expect("failed to parse DATABASE_URL");

    let pool = PoolOptions::new()
        // MySql's normal connection limit is 150 plus 1 superuser connection
        // We don't want to use the whole cap and there may be fuzziness here due to
        // concurrently running tests anyway.
        .max_connections(20)
        // Immediately close master connections. Tokio's I/O streams don't like hopping runtimes.
        .after_release(|_conn, _| Box::pin(async move { Ok(false) }))
        .connect_lazy_with(master_opts);

    let master_pool = match MASTER_POOL.try_insert(pool) {
        Ok(inserted) => inserted,
        Err((existing, pool)) => {
            // Sanity checks.
            assert_eq!(
                existing.connect_options().host,
                pool.connect_options().host,
                "DATABASE_URL changed at runtime, host differs"
            );

            assert_eq!(
                existing.connect_options().database,
                pool.connect_options().database,
                "DATABASE_URL changed at runtime, database differs"
            );

            existing
        }
    };

    let mut conn = master_pool.acquire().await?;

    // language=MySQL
    conn.execute(indoc::indoc!(
        r#"
        CREATE SCHEMA IF NOT EXISTS _sqlx_test;

        CREATE TABLE IF NOT EXISTS _sqlx_test.databases
        (
            db_name    VARCHAR(63) PRIMARY KEY,
            test_path  VARCHAR(255) NOT NULL,
            created_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        "#
    ))
    .await?;

    let db_name = MySql::db_name(args);
    do_cleanup(&mut conn, &db_name).await?;

    query(indoc::indoc!(
        r#"
        INSERT INTO _sqlx_test.databases(db_name, test_path)
        VALUES (?, ?);
        "#
    ))
    .bind(&db_name)
    .bind(args.test_path)
    .execute(&mut *conn)
    .await?;

    conn.execute(format!("CREATE DATABASE {db_name};").as_str())
        .await?;

    Ok(TestContext {
        pool_opts: PoolOptions::new()
            // Don't allow a single test to take all the connections.
            // Most tests shouldn't require more than 5 connections concurrently,
            // or else they're likely doing too much in one test.
            .max_connections(5)
            // Close connections ASAP if left in the idle queue.
            .idle_timeout(Some(Duration::from_secs(1)))
            .parent(master_pool.clone()),
        connect_opts: master_pool
            .connect_options()
            .deref()
            .clone()
            .database(&db_name),
        db_name,
    })
}

async fn do_cleanup(conn: &mut MySqlConnection, db_name: &str) -> Result<(), Error> {
    conn.execute(format!("DROP DATABASE IF EXISTS {db_name};").as_str())
        .await?;

    query(indoc::indoc!(
        r#"
        DELETE
        FROM _sqlx_test.databases
        WHERE db_name = ?
        "#
    ))
    .bind(db_name)
    .execute(&mut *conn)
    .await?;

    Ok(())
}
