use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Heartbeat {
    Ping(Vec<u8>),
    Pong(Vec<u8>),
}

impl Heartbeat {
    pub fn ping(random_string: &str) -> Self {
        Self::Ping(random_string.as_bytes().to_vec())
    }
}
