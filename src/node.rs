use super::error::Result;
use chrono::NaiveDateTime;
use std::net::SocketAddrV4;

/// Provide infrastructure for communication over Frame.
pub struct Node {
    /// Node id
    pub id: i32,
    /// Node's unique name
    pub name: String,
    /// Peer address. Used to identify nodes.
    pub node_addr: SocketAddrV4,
    /// Last communication time.
    pub last_update: NaiveDateTime,

    /* Statistic information. */
    /// Count of sent frames.
    pub frame_sent: usize,
    /// Count of received frames.
    pub frame_recvd: usize,
}
