use crate::{RespArray, RespFrame, RespNull};

use super::{extract_args, validate_command, CommandError, CommandExecutor, Get, Set, RESP_OK};

impl CommandExecutor for Get {
    fn execute(self, backend: &crate::backend::Backend) -> RespFrame {
        match backend.get(&self.key) {
            Some(value) => value,
            None => RespFrame::Null(RespNull),
        }
    }
}

impl CommandExecutor for Set {
    fn execute(self, backend: &crate::backend::Backend) -> RespFrame {
        backend.set(&self.key, self.value);
        RESP_OK.clone()
    }
}

impl TryFrom<RespArray> for Get {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["get"], 1)?;
        let args = extract_args(value, 1)?;

        match args[0] {
            RespFrame::BulkString(ref key) => Ok(Get {
                key: String::from_utf8(key.as_slice().to_vec())?,
            }),
            _ => Err(CommandError::InvalidArgument("Invalid key".into())),
        }
    }
}

impl TryFrom<RespArray> for Set {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["set"], 2)?;
        let mut args = extract_args(value, 1)?.into_iter();

        match (args.next(), args.next()) {
            (Some(RespFrame::BulkString(key)), Some(value)) => Ok(Set {
                key: String::from_utf8(key.to_vec())?,
                value,
            }),
            _ => Err(CommandError::InvalidArgument("Invalid key or value".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::{BulkString, RespDecode};
    use anyhow::Result;

    use super::*;
    #[test]
    fn test_get_try_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::from("*2\r\n$3\r\nget\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;
        let get = Get::try_from(frame).unwrap();
        assert_eq!(get.key, "hello");
        Ok(())
    }

    #[test]
    fn test_set_try_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::from("*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n");

        let frame = RespArray::decode(&mut buf)?;
        let result = Set::try_from(frame)?;
        assert_eq!(result.key, "hello");
        assert_eq!(result.value, BulkString::new("world").into());
        Ok(())
    }

    #[test]
    fn test_set_get_command() -> Result<()> {
        let backend = crate::backend::Backend::new();
        let cmd = Set {
            key: "hello".into(),
            value: BulkString::new("world").into(),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RESP_OK.clone());

        let cmd = Get {
            key: "hello".into(),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, BulkString::new("world").into());

        Ok(())
    }
}
