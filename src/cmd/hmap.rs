use crate::{BulkString, RespArray, RespFrame, RespNull};

use super::{
    extract_args, validate_command, CommandError, CommandExecutor, HGet, HGetAll, HSet, RESP_OK,
};

impl CommandExecutor for HGet {
    fn execute(self, backend: &crate::backend::Backend) -> RespFrame {
        match backend.hget(&self.key, &self.field) {
            Some(value) => value,
            None => RespFrame::Null(RespNull),
        }
    }
}

impl CommandExecutor for HSet {
    fn execute(self, backend: &crate::backend::Backend) -> RespFrame {
        backend.hset(&self.key, &self.field, self.value);
        RESP_OK.clone()
    }
}

impl CommandExecutor for HGetAll {
    fn execute(self, backend: &crate::backend::Backend) -> RespFrame {
        match backend.hgetall(&self.key) {
            Some(hmap) => {
                let mut data: Vec<_> = hmap
                    .iter()
                    .map(|v| (v.key().to_owned(), v.value().clone()))
                    .collect();
                if self.sort {
                    data.sort_by(|a, b| a.0.cmp(&b.0))
                }
                let ret = data
                    .into_iter()
                    .flat_map(|(k, v)| vec![BulkString::from(k.as_ref()).into(), v])
                    .collect::<Vec<RespFrame>>();
                RespArray::new(ret).into()
            }
            None => RespArray::new([]).into(),
        }
    }
}

impl TryFrom<RespArray> for HGet {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["hget"], 2)?;
        let mut args = extract_args(value, 1)?.into_iter();
        match (args.next(), args.next()) {
            (Some(RespFrame::BulkString(key)), Some(RespFrame::BulkString(field))) => Ok(HGet {
                key: String::from_utf8(key.to_vec())?,
                field: String::from_utf8(field.to_vec())?,
            }),
            _ => Err(CommandError::InvalidArgument("invalid key or field".into())),
        }
    }
}

impl TryFrom<RespArray> for HSet {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["hset"], 3)?;
        let mut args = extract_args(value, 1)?.into_iter();
        match (args.next(), args.next(), args.next()) {
            (Some(RespFrame::BulkString(key)), Some(RespFrame::BulkString(field)), Some(value)) => {
                Ok(HSet {
                    key: String::from_utf8(key.to_vec())?,
                    field: String::from_utf8(field.to_vec())?,
                    value,
                })
            }
            _ => Err(CommandError::InvalidArgument(
                "invalid key or field or value".into(),
            )),
        }
    }
}

impl TryFrom<RespArray> for HGetAll {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["hgetall"], 1)?;
        let mut args = extract_args(value, 1)?.into_iter();
        match args.next() {
            Some(RespFrame::BulkString(key)) => Ok(HGetAll {
                key: String::from_utf8(key.to_vec())?,
                sort: false,
            }),
            _ => Err(CommandError::InvalidArgument("invalid key".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backend::Backend, BulkString, RespDecode};
    use anyhow::Result;
    use bytes::BytesMut;

    #[test]
    fn test_hget_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::from("*3\r\n$4\r\nhget\r\n$5\r\nmykey\r\n$7\r\nmyfield\r\n");
        let frame = RespArray::decode(&mut buf)?;
        let hget = HGet::try_from(frame)?;
        assert_eq!(hget.key, "mykey");
        assert_eq!(hget.field, "myfield");
        Ok(())
    }

    #[test]
    fn test_hset_from_resp_array() -> Result<()> {
        let mut buf =
            BytesMut::from("*4\r\n$4\r\nhset\r\n$5\r\nmykey\r\n$7\r\nmyfield\r\n$7\r\nmyvalue\r\n");
        let frame = RespArray::decode(&mut buf)?;
        let hset = HSet::try_from(frame)?;
        assert_eq!(hset.key, "mykey");
        assert_eq!(hset.field, "myfield");
        assert_eq!(hset.value, BulkString::new("myvalue").into());
        Ok(())
    }

    #[test]
    fn test_hgetall_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::from("*2\r\n$7\r\nhgetall\r\n$5\r\nmykey\r\n");
        let frame = RespArray::decode(&mut buf)?;
        let hgetall = HGetAll::try_from(frame)?;
        assert_eq!(hgetall.key, "mykey");
        Ok(())
    }

    #[test]
    fn test_hset_hset_command() -> Result<()> {
        let backend = Backend::new();
        let cmd = HSet {
            key: "hello".into(),
            field: "myfield".into(),
            value: BulkString::new("world").into(),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RESP_OK.clone());

        let cmd = HSet {
            key: "hello".into(),
            field: "myfield1".into(),
            value: BulkString::new("world1").into(),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RESP_OK.clone());

        let cmd = HGet {
            key: "hello".into(),
            field: "myfield".into(),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, BulkString::new("world").into());

        let cmd = HGetAll {
            key: "hello".into(),
            sort: true,
        };

        let expected = RespArray::new([
            BulkString::from("myfield").into(),
            BulkString::from("world").into(),
            BulkString::from("myfield1").into(),
            BulkString::from("world1").into(),
        ]);
        let result = cmd.execute(&backend);
        assert_eq!(result, expected.into());
        Ok(())
    }
}
