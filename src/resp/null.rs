use bytes::BytesMut;

use crate::{RespDecode, RespEncode, RespError};

use super::extract_fixed_data;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RespNull;

// - null: "_\r\n"
impl RespEncode for RespNull {
    fn encode(self) -> Vec<u8> {
        "_\r\n".into()
    }
}

// - null: "_\r\n"
impl RespDecode for RespNull {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "_\r\n", "Null")?;
        Ok(RespNull)
    }
}

#[cfg(test)]
mod tests {
    use crate::RespFrame;

    use super::*;

    #[test]
    fn test_null_encode() {
        let frame: RespFrame = RespNull.into();
        assert_eq!(frame.encode(), b"_\r\n");
    }
}
