use std::net::SocketAddrV4;
use tokio::time::Instant;

/// Provide infrastructure for communication over Frame.
#[derive(Debug)]
pub struct Node {
    /// Node id
    pub id: i32,
    /// Node's unique name
    pub name: String,
    /// Peer address. Used to identify nodes.
    pub node_addr: SocketAddrV4,
    /// Last communication time. Updated by discovery service.
    pub last_update: Instant,

    /* Statistic information. */
    /// Count of sent frames.
    pub frame_sent: usize,
    /// Count of received frames.
    pub frame_recvd: usize,
}

impl Node {
    pub fn new(name: String, addr: SocketAddrV4) -> Self {
        Self {
            id: 0,
            name,
            node_addr: addr,
            last_update: Instant::now(),
            frame_sent: 0,
            frame_recvd: 0,
        }
    }
}
