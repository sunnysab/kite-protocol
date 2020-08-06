mod discovery;
mod event;
mod heartbeat;

pub use discovery::AgentBasic;
pub use heartbeat::Heartbeat;
use serde::{Deserialize, Serialize};

/// Packet payload.
#[derive(Debug, Serialize, Deserialize)]
pub enum Body {
    /// Empty
    Empty,
    /// Used for Agent to send to host, no response expected.
    Discovery(AgentBasic),
    /// Heartbeat frame, response Pong for Ping ~
    Heartbeat(Heartbeat),
}
