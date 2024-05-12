use std::ops::Deref;

use bytes::{Buf, BytesMut};

use crate::{RespDecode, RespEncode, RespError, RespFrame};

use super::{parse_length, BUF_CAP, CRLF_LEN};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RespSet(pub(super) Vec<RespFrame>);

// - set: "~<number-of-elements>\r\n<element-1>...<element-n>"
impl RespEncode for RespSet {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("~{}\r\n", self.len()).into_bytes());
        for value in self.0 {
            buf.extend_from_slice(&value.encode());
        }
        buf
    }
}

// - set: "~<number-of-elements>\r\n<element-1>...<element-n>"
impl RespDecode for RespSet {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let prefix = "~";
        let (end, len) = parse_length(buf, prefix)?;

        let mut newbuf = buf.clone();
        newbuf.advance(end + CRLF_LEN);
        let mut frames = Vec::new();
        for _ in 0..len {
            frames.push(RespFrame::decode(&mut newbuf).map_err(|_| RespError::NotComplete)?);
        }

        buf.advance(buf.len() - newbuf.len());
        Ok(RespSet::new(frames))
    }
}

impl Deref for RespSet {
    type Target = Vec<RespFrame>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RespSet {
    pub fn new(v: impl Into<Vec<RespFrame>>) -> Self {
        RespSet(v.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{BulkString, RespArray};

    use super::*;
    use anyhow::Result;

    #[test]
    fn test_set_encode() {
        let set = RespSet::new([
            RespArray::new([1234.into(), true.into()]).into(),
            BulkString::new("world").into(),
        ]);
        let frame: RespFrame = set.into();
        assert_eq!(
            frame.encode(),
            b"~2\r\n*2\r\n:+1234\r\n#t\r\n$5\r\nworld\r\n"
        )
    }

    #[test]
    fn test_set_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"~2\r\n$3\r\nset\r\n$5\r\nhello\r\n");

        let frame = RespSet::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespSet::new(vec![
                BulkString::new("set").into(),
                BulkString::new("hello").into(),
            ])
        );
        Ok(())
    }
}
