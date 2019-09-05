use crate::mariadb::{
    Capabilities, ColumnDefPacket, ComStmtPrepareOk, Decode, EofPacket, Framed,
};
use crate::io::BufStream;
use tokio::net::TcpStream;

#[derive(Debug, Default)]
pub struct ComStmtPrepareResp {
    pub ok: ComStmtPrepareOk,
    pub param_defs: Option<Vec<ColumnDefPacket>>,
    pub res_columns: Option<Vec<ColumnDefPacket>>,
}

impl ComStmtPrepareResp {
    pub async fn deserialize(stream: &BufStream<TcpStream>, capabilities: Capabilities) -> io::Result<Self> {
        let ok = ComStmtPrepareOk::decode(buf, capabilities)?;

        let param_defs = if ok.params > 0 {
            let mut param_defs = Vec::new();

            for _ in 0..ok.params {
                ctx.next_packet().await?;
                param_defs.push(ColumnDefPacket::decode(&buf, capabilities)?);
            }

            ctx.next_packet().await?;

            if !ctx
                .ctx
                .capabilities
                .contains(Capabilities::CLIENT_DEPRECATE_EOF)
            {
                EofPacket::decode(buf, capabilities)?;
            }

            Some(param_defs)
        } else {
            None
        };

        let res_columns = if ok.columns > 0 {
            let mut res_columns = Vec::new();

            for _ in 0..ok.columns {
                ctx.next_packet().await?;
                res_columns.push(ColumnDefPacket::decode(buf, capabilities)?);
            }

            ctx.next_packet().await?;

            if !capabilities.contains(Capabilities::CLIENT_DEPRECATE_EOF) {
                EofPacket::decode(&mut ctx)?;
            }

            Some(res_columns)
        } else {
            None
        };

        Ok(ComStmtPrepareResp {
            ok,
            param_defs,
            res_columns,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        __bytes_builder,
    };

    #[tokio::test]
    async fn it_decodes_com_stmt_prepare_resp_eof() -> io::Result<()> {
        #[rustfmt::skip]
        let buf = __bytes_builder!(
        // ---------------------------- //
        // Statement Prepared Ok Packet //
        // ---------------------------- //

        // int<3> length
        0u8, 0u8, 0u8,
        // int<1> seq_no
        0u8,
        // int<1> 0x00 COM_STMT_PREPARE_OK header
        0u8,
        // int<4> statement id
        1u8, 0u8, 0u8, 0u8,
        // int<2> number of columns in the returned result set (or 0 if statement does not return result set)
        1u8, 0u8,
        // int<2> number of prepared statement parameters ('?' placeholders)
        1u8, 0u8,
        // string<1> -not used-
        0u8,
        // int<2> number of warnings
        0u8, 0u8,

        // Param column definition

        // ------------------------ //
        // Column Definition packet //
        // ------------------------ //
        // int<3> length
        52u8, 0u8, 0u8,
        // int<1> seq_no
        3u8,
        // string<lenenc> catalog (always 'def')
        3u8, b"def",
        // string<lenenc> schema
        4u8, b"test",
        // string<lenenc> table alias
        5u8, b"users",
        // string<lenenc> table
        5u8, b"users",
        // string<lenenc> column alias
        8u8, b"username",
        // string<lenenc> column
        8u8, b"username",
        // int<lenenc> length of fixed fields (=0xC)
        0x0C_u8,
        // int<2> character set number
        8u8, 0u8,
        // int<4> max. column size
        0xFF_u8, 0xFF_u8, 0u8, 0u8,
        // int<1> Field types
        0xFC_u8,
        // int<2> Field detail flag
        0x11_u8, 0x10_u8,
        // int<1> decimals
        0u8,
        // int<2> - unused -
        0u8, 0u8,

        // Result column definitions

        // ------------------------ //
        // Column Definition packet //
        // ------------------------ //
        // int<3> length
        52u8, 0u8, 0u8,
        // int<1> seq_no
        3u8,
        // string<lenenc> catalog (always 'def')
        3u8, b"def",
        // string<lenenc> schema
        4u8, b"test",
        // string<lenenc> table alias
        5u8, b"users",
        // string<lenenc> table
        5u8, b"users",
        // string<lenenc> column alias
        8u8, b"username",
        // string<lenenc> column
        8u8, b"username",
        // int<lenenc> length of fixed fields (=0xC)
        0x0C_u8,
        // int<2> character set number
        8u8, 0u8,
        // int<4> max. column size
        0xFF_u8, 0xFF_u8, 0u8, 0u8,
        // int<1> Field types
        0xFC_u8,
        // int<2> Field detail flag
        0x11_u8, 0x10_u8,
        // int<1> decimals
        0u8,
        // int<2> - unused -
        0u8, 0u8
        );

        let message = ComStmtPrepareResp::deserialize(&mut buf, Capabilities::CLIENT_PROTOCOL_41).await?;

        Ok(())
    }

    #[tokio::test]
    async fn it_decodes_com_stmt_prepare_resp() -> io::Result<()> {
        #[rustfmt::skip]
            let buf = __bytes_builder!(
        // ---------------------------- //
        // Statement Prepared Ok Packet //
        // ---------------------------- //

        // int<3> length
        0u8, 0u8, 0u8,
        // int<1> seq_no
        0u8,
        // int<1> 0x00 COM_STMT_PREPARE_OK header
        0u8,
        // int<4> statement id
        1u8, 0u8, 0u8, 0u8,
        // int<2> number of columns in the returned result set (or 0 if statement does not return result set)
        1u8, 0u8,
        // int<2> number of prepared statement parameters ('?' placeholders)
        1u8, 0u8,
        // string<1> -not used-
        0u8,
        // int<2> number of warnings
        0u8, 0u8,

        // Param column definition

        // ------------------------ //
        // Column Definition packet //
        // ------------------------ //
        // int<3> length
        52u8, 0u8, 0u8,
        // int<1> seq_no
        3u8,
        // string<lenenc> catalog (always 'def')
        3u8, b"def",
        // string<lenenc> schema
        4u8, b"test",
        // string<lenenc> table alias
        5u8, b"users",
        // string<lenenc> table
        5u8, b"users",
        // string<lenenc> column alias
        8u8, b"username",
        // string<lenenc> column
        8u8, b"username",
        // int<lenenc> length of fixed fields (=0xC)
        0x0C_u8,
        // int<2> character set number
        8u8, 0u8,
        // int<4> max. column size
        0xFF_u8, 0xFF_u8, 0u8, 0u8,
        // int<1> Field types
        0xFC_u8,
        // int<2> Field detail flag
        0x11_u8, 0x10_u8,
        // int<1> decimals
        0u8,
        // int<2> - unused -
        0u8, 0u8,

        // ---------- //
        // EOF Packet //
        // ---------- //
        // int<3> length
        5u8, 0u8, 0u8,
        // int<1> seq_no
        6u8,
        // int<1> 0xfe : EOF header
        0xFE_u8,
        // int<2> warning count
        0u8, 0u8,
        // int<2> server status
        34u8, 0u8,

        // Result column definitions

        // ------------------------ //
        // Column Definition packet //
        // ------------------------ //
        // int<3> length
        52u8, 0u8, 0u8,
        // int<1> seq_no
        3u8,
        // string<lenenc> catalog (always 'def')
        3u8, b"def",
        // string<lenenc> schema
        4u8, b"test",
        // string<lenenc> table alias
        5u8, b"users",
        // string<lenenc> table
        5u8, b"users",
        // string<lenenc> column alias
        8u8, b"username",
        // string<lenenc> column
        8u8, b"username",
        // int<lenenc> length of fixed fields (=0xC)
        0x0C_u8,
        // int<2> character set number
        8u8, 0u8,
        // int<4> max. column size
        0xFF_u8, 0xFF_u8, 0u8, 0u8,
        // int<1> Field types
        0xFC_u8,
        // int<2> Field detail flag
        0x11_u8, 0x10_u8,
        // int<1> decimals
        0u8,
        // int<2> - unused -
        0u8, 0u8,

        // ---------- //
        // EOF Packet //
        // ---------- //
        // int<3> length
        5u8, 0u8, 0u8,
        // int<1> seq_no
        6u8,
        // int<1> 0xfe : EOF header
        0xFE_u8,
        // int<2> warning count
        0u8, 0u8,
        // int<2> server status
        34u8, 0u8
        );

        let message = ComStmtPrepareResp::deserialize(&buf, Capabilities::CLIENT_PROTOCOL_41).await?;

        Ok(())
    }
}
