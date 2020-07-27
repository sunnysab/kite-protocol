use crate::error::{Result, TaskError};
use crate::node::Node;
use crate::protocol::Frame;
use crate::services::Body;
use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::{
    udp::{RecvHalf, SendHalf},
    UdpSocket,
};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    oneshot, Mutex,
};
use tokio::time::{Duration, Instant};

pub type SeqType = u32;

#[derive(Debug)]
pub enum HostError {
    NoCarawler,
    InitNeeded,
    Timeout,
}

impl From<HostError> for TaskError {
    fn from(e: HostError) -> Self {
        TaskError::Controller(e)
    }
}

/// Controller, as server side node
pub struct Host {
    /// Crawler nodes.
    nodes: Arc<Mutex<Vec<Node>>>,
    /// Local address and port
    addr: String,
    /// Sender, send frames to send loop
    sender: Option<Sender<(Frame, SocketAddrV4)>>,
    /// Queue, which queued requests which not being responded at the moment. <br>
    /// When `request()` sent a frame, it also adds a `oneshot::Sender` here, so that `receive_loop()`
    /// can find the requester and post response for it.
    wait_queue: Arc<Mutex<HashMap<SeqType, oneshot::Sender<(Frame, SocketAddrV4)>>>>,

    /* Statistic information. */
    /// Count of sent frames.
    pub frame_sent: usize,
    /// Count of received frames.
    pub frame_recv: usize,
}

impl Host {
    /// Create and initialize a controller node.
    pub async fn new(port: u16) -> Result<Self> {
        let local_addr = format!("0.0.0.0:{}", port);

        Ok(Self {
            nodes: Arc::new(Mutex::new(vec![])),
            addr: local_addr,
            sender: None,
            wait_queue: Arc::new(Mutex::new(HashMap::new())),
            frame_sent: 0,
            frame_recv: 0,
        })
    }

    async fn update_node(nodes: Arc<Mutex<Vec<Node>>>, addr: &SocketAddrV4) {
        let mut nodes = nodes.lock().await;
        for i in nodes.iter_mut() {
            if i.node_addr == *addr {
                i.last_update = Instant::now();
                return;
            }
        }
        // If the current is a new node
        let pos = nodes.len();
        nodes.insert(pos, Node::new(addr.clone()));
        println!("Nodes: {:?}", nodes);
    }

    /// Choose an available node randomly.
    async fn choose_node(&mut self, _body: &Body) -> Option<SocketAddrV4> {
        let mut nodes = self.nodes.lock().await;
        if nodes.len() != 0 {
            // TODO: 随机选择节点
            // TODO: 淘汰过旧节点
            nodes[0].last_update = Instant::now();
            return Some(nodes[0].node_addr);
        }
        return None;
    }

    /// Receive frames and post them to where they should go :D
    async fn receive_loop(
        mut recv_socket: RecvHalf,
        nodes: Arc<Mutex<Vec<Node>>>,
        wait_queue: Arc<Mutex<HashMap<SeqType, oneshot::Sender<(Frame, SocketAddrV4)>>>>,
    ) {
        // Alloc 512K for Udp receive buffer.
        let mut buffer = vec![0u8; 512 * 1024];

        loop {
            // Wait for new udp packet
            if let Ok((size, SocketAddr::V4(addr))) = recv_socket.recv_from(&mut buffer).await {
                // Read and deserialize the frame
                let frame = Frame::read(&mut buffer[..size]).unwrap();
                let seq = frame.seq;

                // Refresh node list
                Self::update_node(Arc::clone(&nodes), &addr).await;

                // Find receiver and post the new response to him.
                let mut queue = wait_queue.lock().await;
                if let Some(target) = queue.remove(&seq) {
                    target.send((frame, addr));
                }
            }
        }
    }

    /// The send loop, receive protocol requests and post to crawlers over UDP.
    async fn send_loop(mut send_socket: SendHalf, mut rx: Receiver<(Frame, SocketAddrV4)>) {
        while let Some((frame, peer)) = rx.recv().await {
            let peer = SocketAddr::V4(peer);

            send_socket.send_to(frame.write().as_slice(), &peer).await;
        }
    }

    /// Bind socket and start
    pub async fn start(&mut self) -> Result<()> {
        // Create channel to send/recv task
        let (tx, rx) = mpsc::channel::<(Frame, SocketAddrV4)>(100);

        // Create Udp socket and bind local address.
        let socket = UdpSocket::bind(&self.addr).await?;
        let (recv_socket, send_socket) = socket.split();

        // Spawn send/recv IO tasks.
        tokio::spawn(Self::receive_loop(
            recv_socket,
            Arc::clone(&self.nodes),
            Arc::clone(&self.wait_queue),
        ));
        tokio::spawn(Self::send_loop(send_socket, rx));
        self.sender = Some(tx);

        Ok(())
    }

    /// Send a request and return the response.
    pub async fn request(&mut self, body: Body, timeout: u64) -> Result<Body> {
        let node = self.choose_node(&body).await.unwrap();

        if let Some(sender) = &mut self.sender {
            let (tx, rx) = oneshot::channel::<(Frame, SocketAddrV4)>();
            let frame = Frame::new(body).unwrap();
            let seq = frame.seq;

            sender.send((frame, node)).await;

            let mut wait_queue = self.wait_queue.lock().await;
            wait_queue.insert(seq, tx);
            // Release lock immediately
            drop(wait_queue);

            let response = tokio::time::timeout(Duration::from_millis(timeout), rx).await;
            return match response {
                Ok(Ok((frame, _))) => Ok(frame.body),
                _ => Err(HostError::Timeout.into()),
            };
        }
        return Err(HostError::NoCarawler.into());
    }
}
