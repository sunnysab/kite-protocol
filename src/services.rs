mod event;
mod heartbeat;

pub use heartbeat::Heartbeat;
use serde::{Deserialize, Serialize};

/// Packet payload.
#[derive(Debug, Serialize, Deserialize)]
pub enum Body {
    Heartbeat(Heartbeat),
}
