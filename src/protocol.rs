use crate::error::TaskError;
use crate::services::Body;
use bincode::deserialize;
use bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};
use zstd::block::{compress, decompress};

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
    static ref LAST_SEQ: AtomicU32 = AtomicU32::new(1 as u32);
}

const FLAG_COMPRESSED: u8 = 0x01;

#[derive(Debug, Serialize, Deserialize)]
pub struct Frame {
    /// 4 bytes package length. Use u32 because size of usize
    /// changes between 32bit and 64bit targets.
    pub size: u16,
    /// Packet sequence number.
    pub seq: u32,
    /// Packet response sequence number.
    pub ack: u32,
    /// Flag
    pub flag: u8,
    /// Request or response Body.
    pub body: Body,
}

impl Frame {
    /// Create a new packet
    pub fn new_request(b: Body) -> Result<Frame> {
        let seq = LAST_SEQ.fetch_add(1, Ordering::Relaxed);

        Ok(Frame {
            size: 0,
            seq: seq,
            ack: 0,
            body: b,
            flag: 0,
        })
    }

    /// Create a new packet
    pub fn new_response(b: Body, ack: u32) -> Result<Frame> {
        let seq = LAST_SEQ.fetch_add(1, Ordering::Relaxed);

        Ok(Frame {
            size: 0,
            seq,
            ack,
            body: b,
            flag: 0,
        })
    }

    /// Write packet to a Vec<u8> buffer.
    pub fn write(&mut self) -> Vec<u8> {
        let mut buffer = Vec::new();
        let origin_payload = bincode::serialize(&self.body).unwrap();
        // Length smaller than 32 does not need to be compressed
        let payload = if origin_payload.len() > 32 {
            let compressed_payload = compress(origin_payload.as_slice(), 0);
            // Write compressed payload if compress successfully and the payload compressed is smaller.
            if let Ok(compressed_payload_binary) = compressed_payload {
                self.flag |= FLAG_COMPRESSED;
                compressed_payload_binary
            } else {
                origin_payload
            }
        } else {
            origin_payload
        };
        self.size = (15 + payload.len()) as u16;

        // Note: write function always returns Result::Ok, so we can unwrap directly.
        buffer.write(HEADER_MAGIC_HEADER).unwrap();
        buffer.put_u16_le(self.size);
        buffer.put_u32(self.seq);
        buffer.put_u32(self.ack);
        buffer.put_u8(self.flag);
        buffer.write(payload.as_slice()).unwrap();

        buffer
    }

    /// Read a packet from buffer.
    #[allow(dead_code)]
    pub fn read(buffer: &mut [u8]) -> Result<Self> {
        // If packet size < minimun size, throw up an error.
        if buffer.len() < 15 {
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
        let ack = fields.get_u32_le();
        let flag = fields.get_u8();
        // If the compress flag set, decompress first.
        let body: Body = if flag & FLAG_COMPRESSED != 0 {
            let decompressed_payload = decompress(fields, 512 * 1024).unwrap();
            deserialize(decompressed_payload.as_slice()).unwrap()
        } else {
            deserialize(fields).unwrap()
        };
        let frame = Self {
            size: buffer.len() as u16,
            seq,
            ack,
            flag,
            body,
        };
        Ok(frame)
    }
}
