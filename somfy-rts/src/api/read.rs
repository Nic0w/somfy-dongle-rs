use std::io::Cursor;

use serde_json::{error::Category, Value};
use thiserror::Error;
use tokio::io::AsyncReadExt;

use bytes::{Buf, BytesMut};
use tokio_serial::SerialStream;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Need more bytes.")]
    Incomplete,

    #[error("Failed to decode serial data as text.")]
    BadEncoding,

    #[error("Expected to read more bytes but reached EOS.")]
    EndOfStream,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn get_byte(cursor: &mut Cursor<&[u8]>) -> Option<u8> {
    cursor.has_remaining().then(|| cursor.get_u8())
}

pub async fn try_read_stuff<'c, T>(
    reader: &'c mut SerialStream,
    buffer: &'c mut BytesMut,
    mut response: Box<dyn Response<T>>,
) -> Result<T, Error> {
    loop {
        let mut cursor = Cursor::new(&buffer[..]);

        if let Ok(res) = response.read(&mut cursor) {
            let consumed = cursor.position() as usize;
            buffer.advance(consumed);

            return Ok(res);
        }

        if reader.readable().await.is_ok() {
            let new_bytes = reader.read_buf(buffer).await.map_err(Error::Io)?;

            if new_bytes == 0 {
                return Err(Error::EndOfStream);
            }
        }
    }
}

pub trait Response<T> {
    fn read(&mut self, cursor: &mut Cursor<&[u8]>) -> Result<T, Error>;
}

pub struct LinesResponse {
    nb_lines: u8,
}

impl LinesResponse {
    pub fn new(nb_lines: u8) -> Box<Self> {
        Box::new(LinesResponse { nb_lines })
    }

    fn read_line<'buf>(cursor: &mut Cursor<&'buf [u8]>) -> Result<&'buf str, Error> {
        let start = cursor.position() as usize;

        let mut next_byte = || get_byte(cursor).ok_or(Error::Incomplete);

        let mut len = 0;

        loop {
            match next_byte()? {
                0x0D if next_byte()? == 0x0A => break, //new line
                _ => len += 1,
            }
        }
        cursor.set_position((2 + start + len as usize) as u64);

        let slice = &cursor.get_ref()[start..start + (len as usize)];

        std::str::from_utf8(slice).or(Err(Error::BadEncoding))
    }
}

impl Response<Vec<String>> for LinesResponse {
    fn read(&mut self, cursor: &mut Cursor<&[u8]>) -> Result<Vec<String>, Error> {
        let mut lines = Vec::new();

        let mut n = self.nb_lines;

        while n > 0 {
            lines.push(Self::read_line(cursor)?.to_string());

            n -= 1;
        }

        Ok(lines)
    }
}

pub struct JsonResponse;

impl JsonResponse {}

impl Response<Value> for JsonResponse {
    fn read(&mut self, cursor: &mut Cursor<&[u8]>) -> Result<Value, Error> {
        let start = cursor.position() as usize;
        let len = cursor.get_ref().len();

        let available = &cursor.get_ref()[start..start + len];

        let string = std::str::from_utf8(available).unwrap();

        match serde_json::from_str::<Value>(string) {
            Ok(v) => Ok(v),
            Err(e) if e.classify() == Category::Eof => Err(Error::Incomplete),
            _ => unreachable!(),
        }
    }
}
