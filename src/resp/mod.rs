mod array;
mod bool;
mod bulk_string;
mod double;
mod frame;
mod integer;
mod map;
mod null;
mod set;
mod simple_error;
mod simple_string;

use anyhow::Result;

use bytes::{Buf, BytesMut};
use enum_dispatch::enum_dispatch;
use std::string::FromUtf8Error;
use thiserror::Error;

pub use self::{
    array::{RespArray, RespNullArray},
    bulk_string::BulkString,
    bulk_string::RespNullBulkString,
    frame::RespFrame,
    map::RespMap,
    null::RespNull,
    set::RespSet,
    simple_error::SimpleError,
    simple_string::SimpleString,
};

const BUF_CAP: usize = 4096;
const CRLF_LEN: usize = 2;

#[derive(Error, Debug, PartialEq)]
pub enum RespError {
    #[error("Invalid frame: {0}")]
    InvalidFrame(String),
    #[error("Invalid frame type: {0}")]
    InvalidFrameType(String),
    #[error("Invalid frame length: {0}")]
    InvalidFrameLength(isize),
    #[error("Frame is not complete")]
    NotComplete,
    #[error("Parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Utf8 error: {0}")]
    Utf8Error(#[from] FromUtf8Error),
    #[error("Parse float error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),
}

#[enum_dispatch]
pub trait RespEncode {
    fn encode(self) -> Vec<u8>;
}

pub trait RespDecode: Sized {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError>;
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

    buf.windows(2)
        .position(|pair| pair == [b'\r', b'\n'])
        .ok_or(RespError::NotComplete)
}

fn extract_fixed_data(
    buf: &mut BytesMut,
    expect: &str,
    expect_type: &str,
) -> Result<(), RespError> {
    if buf.len() < expect.len() {
        return Err(RespError::NotComplete);
    }
    // let data = buf.split_to(expect.len());
    let data = &buf[..expect.len()];
    if data != expect.as_bytes() {
        return Err(RespError::InvalidFrame(format!("expect {:?}", expect)));
    }

    if !buf.starts_with(expect.as_bytes()) {
        return Err(RespError::InvalidFrameType(format!(
            "expect: {}, got: {:?}",
            expect_type, buf
        )));
    }

    buf.advance(expect.len());
    Ok(())
}

fn parse_length(buf: &mut BytesMut, prefix: &str) -> Result<(usize, usize), RespError> {
    let end = extract_simpe_frame_data(buf, prefix)?;
    let s = String::from_utf8_lossy(&buf[prefix.len()..end]);
    Ok((end, s.parse()?))
}

#[cfg(test)]
mod tests {
    // TODO: Add tests
}
