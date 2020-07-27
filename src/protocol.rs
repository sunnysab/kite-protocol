use crate::error::TaskError;
use crate::services::Body;
use bincode::deserialize;
use bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};

type Result<T> = std::result::Result<T, ProtocolError>;

#[derive(Debug)]
pub enum ProtocolError {
    TooSmall(usize),
    Serialize,
    Deserialize,
    MismatchSize,
    MismatchMagic,
}

impl From<ProtocolError> for TaskError {
    fn from(e: ProtocolError) -> Self {
        TaskError::Protocol(e)
    }
}

const HEADER_MAGIC_HEADER: &'static [u8; 4] = &[0x07, 0x55, 0xAA, 0xB3];
lazy_static! {
    static ref LAST_SEQ: AtomicU32 = AtomicU32::new(0 as u32);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Frame {
    /// 4 bytes package length. Use u32 because size of usize
    /// changes between 32bit and 64bit targets.
    pub size: u16,
    /// Packet sequence number.
    pub seq: u32,
    pub body: Body,
}

impl Frame {
    /// Create a new packet
    pub fn new(b: Body) -> Result<Frame> {
        let seq = LAST_SEQ.fetch_add(1, Ordering::Relaxed);

        Ok(Frame {
            size: 0,
            seq: seq,
            body: b,
        })
    }

    /// Create a new packet
    pub fn new_response(b: Body, seq: u32) -> Result<Frame> {
        Ok(Frame {
            size: 0,
            seq,
            body: b,
        })
    }

    /// Write packet to a Vec<u8> buffer.
    pub fn write(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        let payload = bincode::serialize(&self.body).unwrap();
        let size = (10 + payload.len()) as u16;

        // Note: write function always returns Result::Ok, so we can unwrap directly.
        buffer.write(HEADER_MAGIC_HEADER).unwrap();
        buffer.put_u16_le(size);
        buffer.put_u32(self.seq);
        // buffer.put_u8(self.pack_type);
        // buffer.put_u8(self.flag);
        buffer.write(payload.as_slice()).unwrap();
        buffer
    }

    /// Read a packet from buffer.
    #[allow(dead_code)]
    pub fn read(buffer: &mut [u8]) -> Result<Self> {
        // If packet size < minimun size, throw up an error.
        if buffer.len() < 10 {
            return Err(ProtocolError::TooSmall(buffer.len()));
        }
        // Check frame magic header.
        if &buffer[..4] != HEADER_MAGIC_HEADER {
            return Err(ProtocolError::MismatchMagic);
        }
        let mut fields = &buffer[4..];
        // buffer.len() as actual size, and fileds.get_u16_le() as expect size.
        if buffer.len() != fields.get_u16_le() as usize {
            return Err(ProtocolError::MismatchSize);
        }
        let seq = fields.get_u32_le();
        let body: Body = deserialize(fields).unwrap();
        let frame = Self {
            size: buffer.len() as u16,
            seq,
            body,
        };
        Ok(frame)
    }
}
