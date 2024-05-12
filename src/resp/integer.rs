use bytes::BytesMut;

use crate::{RespDecode, RespEncode, RespError};

use super::extract_simpe_frame_data;

// - integer: ":[<+|->]<value>\r\n"
impl RespEncode for i64 {
    fn encode(self) -> Vec<u8> {
        let sign = if self < 0 { "" } else { "+" };
        format!(":{}{}\r\n", sign, self).into_bytes()
    }
}

// - integer: ":[<+|->]<value>\r\n"
impl RespDecode for i64 {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simpe_frame_data(buf, ":")?;
        let data = buf.split_to(end + 2);
        let s = String::from_utf8_lossy(&data[1..end]);
        Ok(s.parse()?)
    }
}

#[cfg(test)]
mod tests {
    use crate::RespFrame;

    use super::*;

    #[test]
    fn test_integer_encode() {
        let frame: RespFrame = 42.into();
        assert_eq!(frame.encode(), b":+42\r\n");
        let frame: RespFrame = (-42).into();
        assert_eq!(frame.encode(), b":-42\r\n");
    }
}
