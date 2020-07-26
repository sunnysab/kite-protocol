use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Heartbeat {
    Ping(Vec<u8>),
    Pong(Vec<u8>),
}
