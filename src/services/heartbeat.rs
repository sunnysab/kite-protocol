use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Heartbeat {
    Ping(Vec<u8>),
    Pong(Vec<u8>),
}

impl Heartbeat {
    pub fn ping(size: usize) -> Self {
        // Limit the size of ping packets no more than 255
        let size = if size > 255 { 255 } else { size };

        let mut rng = thread_rng(); // Random generator.
        let mut buffer = vec![0u8; size];
        rng.fill_bytes(&mut buffer);

        Self::Ping(buffer)
    }

    /// Generate a pong packet for ping :D
    pub fn pong(self) -> Self {
        match self {
            Self::Ping(payload) => Self::Pong(payload),
            Self::Pong(payload) => Self::Pong(payload),
        }
    }
}
