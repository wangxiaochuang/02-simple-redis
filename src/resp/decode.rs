use crate::{
    BulkString, RespArray, RespDecode, RespError, RespFrame, RespMap, RespNull, RespNullArray,
    RespNullBulkString, RespSet, SimpleError, SimpleString,
};
use bytes::{Buf, BytesMut};

use super::extract_fixed_data;

const CRLF_LEN: usize = 2;

impl RespDecode for RespFrame {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let mut iter = buf.iter().peekable();
        match iter.peek() {
            Some(b'+') => {
                let frame = SimpleString::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'-') => {
                let frame = SimpleError::decode(buf)?;
                Ok(frame.into())
            }
            Some(b':') => {
                let frame = i64::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'$') => match RespNullBulkString::decode(buf) {
                Ok(frame) => Ok(frame.into()),
                Err(RespError::NotComplete) => Err(RespError::NotComplete),
                Err(_) => {
                    let frame = BulkString::decode(buf)?;
                    Ok(frame.into())
                }
            },
            Some(b'*') => match RespNullArray::decode(buf) {
                Ok(frame) => Ok(frame.into()),
                Err(RespError::NotComplete) => Err(RespError::NotComplete),
                Err(_) => {
                    let frame = RespArray::decode(buf)?;
                    Ok(frame.into())
                }
            },
            Some(b'_') => {
                let frame = RespNull::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'#') => {
                let frame = bool::decode(buf)?;
                Ok(frame.into())
            }
            Some(b',') => {
                let frame = f64::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'%') => {
                let frame = RespMap::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'~') => {
                let frame = RespSet::decode(buf)?;
                Ok(frame.into())
            }
            None => Err(RespError::NotComplete),
            _ => Err(RespError::InvalidFrameType(format!(
                "expect_length: unknown frame type: {:?}",
                buf
            ))),
        }
    }
}

fn extract_simpe_frame_data(buf: &mut BytesMut, prefix: &str) -> Result<usize, RespError> {
    if buf.len() < 3 {
        return Err(RespError::NotComplete);
    }
    if !buf.starts_with(prefix.as_bytes()) {
        return Err(RespError::InvalidFrameType(format!(
            "expect: SimpleString({}), got: {:?}",
            prefix, buf
        )));
    }

    let mut end = 0;
    for i in 0..buf.len() - 1 {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            end = i;
            break;
        }
    }

    if end == 0 {
        Err(RespError::NotComplete)
    } else {
        Ok(end)
    }
}
// - simple string: "+OK\r\n"
impl RespDecode for SimpleString {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simpe_frame_data(buf, "+")?;
        let data = buf.split_to(end + 2);
        let s = String::from_utf8_lossy(&data[1..end]);
        Ok(SimpleString::new(s))
    }
}

// - simple error: "-Error message\r\n"
impl RespDecode for SimpleError {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simpe_frame_data(buf, "-")?;
        let data = buf.split_to(end + 2);
        let s = String::from_utf8_lossy(&data[1..end]);
        Ok(SimpleError::new(s))
    }
}

// - null: "_\r\n"
impl RespDecode for RespNull {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "_\r\n", "Null")?;
        Ok(RespNull)
    }
}

// - null array: "*-1\r\n"
impl RespDecode for RespNullArray {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "*-1\r\n", "NullArray")?;
        Ok(RespNullArray)
    }
}

// - null bulk string: "$-1\r\n"
impl RespDecode for RespNullBulkString {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "$-1\r\n", "NullBulkString")?;
        Ok(RespNullBulkString)
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

// - boolean: "#<t|f>\r\n"
impl RespDecode for bool {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        match extract_fixed_data(buf, "#t\r\n", "Bool") {
            Ok(_) => Ok(true),
            Err(_) => match extract_fixed_data(buf, "#f\r\n", "Bool") {
                Ok(_) => Ok(false),
                Err(e) => Err(e),
            },
        }
    }
}

// - bulk string: "$<length>\r\n<data>\r\n"
impl RespDecode for BulkString {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let prefix = "$";
        let (end, len) = parse_length(buf, prefix)?;
        let remained = &buf[end + CRLF_LEN..];
        if remained.len() < len + CRLF_LEN {
            return Err(RespError::NotComplete);
        }
        buf.advance(end + CRLF_LEN);

        let data = buf.split_to(len + CRLF_LEN);
        Ok(BulkString::new(data[..len].to_vec()))
    }
}

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
impl RespDecode for RespArray {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let prefix = "*";
        let (end, len) = parse_length(buf, prefix)?;
        let mut newbuf = buf.clone();
        newbuf.advance(end + CRLF_LEN);
        let mut frames = Vec::with_capacity(len);
        for _ in 0..len {
            let res = RespFrame::decode(&mut newbuf).map_err(|_| RespError::NotComplete)?;
            frames.push(res);
        }
        buf.advance(buf.len() - newbuf.len());
        Ok(RespArray::new(frames))
    }
}

impl RespDecode for f64 {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simpe_frame_data(buf, ",")?;
        let data = buf.split_to(end + 2);
        let s = String::from_utf8_lossy(&data[1..end]);
        Ok(s.parse()?)
    }
}

// - map: "%<number-of-entries>\r\n<key-1><value-1>...<key-n><value-n>"
impl RespDecode for RespMap {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let prefix = "%";
        let (end, len) = parse_length(buf, prefix)?;
        let mut newbuf = buf.clone();
        newbuf.advance(end + CRLF_LEN);

        let mut frames = RespMap::new();
        for _ in 0..len {
            let key = SimpleString::decode(&mut newbuf).map_err(|_| RespError::NotComplete)?;
            let value = RespFrame::decode(&mut newbuf).map_err(|_| RespError::NotComplete)?;
            frames.insert(key.0, value);
        }

        buf.advance(buf.len() - newbuf.len());

        Ok(frames)
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

fn parse_length(buf: &mut BytesMut, prefix: &str) -> Result<(usize, usize), RespError> {
    let end = extract_simpe_frame_data(buf, prefix)?;
    let s = String::from_utf8_lossy(&buf[prefix.len()..end]);
    Ok((end, s.parse()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use bytes::BufMut;

    #[test]
    fn test_simple_string_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"+OK\r\n");

        let frame = SimpleString::decode(&mut buf)?;
        assert_eq!(frame, SimpleString::new("OK"));

        buf.extend_from_slice(b"+hello\r");

        let ret = SimpleString::decode(&mut buf);
        assert_eq!(ret, Err(RespError::NotComplete));

        buf.put_u8(b'\n');
        let frame = SimpleString::decode(&mut buf)?;
        assert_eq!(frame, SimpleString::new("hello"));

        Ok(())
    }

    #[test]
    fn test_simple_error_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"-Error message\r\n");

        let frame = SimpleError::decode(&mut buf)?;
        assert_eq!(frame, SimpleError::new("Error message"));
        Ok(())
    }

    #[test]
    fn test_boolean_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"#t\r\n");

        let frame = bool::decode(&mut buf)?;
        assert!(frame);

        buf.extend_from_slice(b"#f\r\n");

        let frame = bool::decode(&mut buf)?;
        assert!(!frame);

        buf.extend_from_slice(b"#f\r");
        let ret = bool::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);

        buf.put_u8(b'\n');
        let frame = bool::decode(&mut buf)?;
        assert!(!frame);

        Ok(())
    }

    #[test]
    fn test_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();

        buf.extend_from_slice(b"*2\r\n+set\r\n+hello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespArray::new(vec![
                SimpleString::new("set").into(),
                SimpleString::new("hello").into()
            ])
        );

        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespArray::new(vec![
                BulkString::new("set").into(),
                BulkString::new("hello").into()
            ])
        );

        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n");
        let ret = RespArray::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);

        buf.extend_from_slice(b"$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespArray::new([
                BulkString::new("set").into(),
                BulkString::new("hello").into()
            ])
        );

        Ok(())
    }

    #[test]
    fn test_double_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b",123.45\r\n");

        let frame = f64::decode(&mut buf)?;
        assert_eq!(frame, 123.45);

        buf.extend_from_slice(b",+1.23456e-9\r\n");
        let frame = f64::decode(&mut buf)?;
        assert_eq!(frame, 1.23456e-9);

        Ok(())
    }

    #[test]
    fn test_map_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"%2\r\n+hello\r\n$5\r\nworld\r\n+foo\r\n$3\r\nbar\r\n");

        let frame = RespMap::decode(&mut buf)?;
        let mut map = RespMap::new();
        map.insert("hello".to_string(), BulkString::new("world").into());
        map.insert("foo".to_string(), BulkString::new("bar").into());
        assert_eq!(frame, map);
        Ok(())
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
