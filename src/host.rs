use crate::error::{Result, TaskError};
use crate::node::Node;
use crate::protocol::Frame;
use crate::services::Body;
use log::{error, info, warn};
use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::udp::{RecvHalf, SendHalf};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot, Mutex};
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
#[derive(Clone)]
pub struct Host {
    /// Crawler nodes.
    nodes: Arc<Mutex<Vec<Node>>>,
    /// Local address and port
    addr: String,
    /// Sender, send frames to send loop
    sender: Option<mpsc::Sender<(Frame, SocketAddrV4)>>,
    /// Queue, which queued requests which not being responded at the moment. <br>
    /// When `request()` sent a frame, it also adds a `oneshot::Sender` here, so that `receiver_loop()`
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
    pub async fn new(local_addr: &str, port: u16) -> Result<Self> {
        let local_addr = format!("{}:{}", local_addr, port);

        Ok(Self {
            nodes: Arc::new(Mutex::new(vec![])),
            addr: local_addr,
            sender: None,
            wait_queue: Arc::new(Mutex::new(HashMap::new())),
            frame_sent: 0,
            frame_recv: 0,
        })
    }

    /// Update agent list, add agent if not exist, and TODO: remove old agents.
    async fn update_node(nodes: Arc<Mutex<Vec<Node>>>, addr: &SocketAddrV4) {
        // Acquire the lock to modify agent node list.
        let mut nodes = nodes.lock().await;
        for each_node in nodes.iter_mut() {
            if each_node.node_addr == *addr {
                each_node.last_update = Instant::now();
                return;
            }
        }
        // The operation of inserting to position 0 is simple.
        // Maybe it is costly, but insertion doesn't happen often.
        info!("Add new agent node: {}", addr.to_string());
        nodes.insert(0, Node::new(addr.clone()));
    }

    /// Choose an available node randomly.
    async fn choose_node(&mut self, _body: &Body) -> Option<SocketAddrV4> {
        let mut nodes = self.nodes.lock().await;
        if nodes.len() != 0 {
            // TODO: 随机选择节点
            // TODO: 淘汰过旧节点（最好单独开个任务做）
            // 如果节点实际失效，下面一行代码可能导致失效节点不会被删除
            nodes[0].last_update = Instant::now();
            return Some(nodes[0].node_addr);
        }
        return None;
    }

    /// Receive frames and post them to where they should go :D
    async fn receiver_loop(
        mut recv_socket: RecvHalf,
        nodes: Arc<Mutex<Vec<Node>>>,
        wait_queue: Arc<Mutex<HashMap<SeqType, oneshot::Sender<(Frame, SocketAddrV4)>>>>,
    ) {
        // Alloc 512K for Udp receive buffer.
        let mut buffer = vec![0u8; 512 * 1024];

        info!("Start listening...");
        // Wait for new udp packet
        while let Ok((size, SocketAddr::V4(addr))) = recv_socket.recv_from(&mut buffer).await {
            // Read and deserialize the frame
            match Frame::read(&mut buffer[..size]) {
                Ok(frame) => {
                    let seq = frame.seq;

                    // Refresh node list
                    Self::update_node(Arc::clone(&nodes), &addr).await;

                    // Find receiver and post the new response to him.
                    let mut queue = wait_queue.lock().await;
                    if let Some(target) = queue.remove(&seq) {
                        target.send((frame, addr));
                    }
                }
                Err(e) => warn!("Failed to unpack frame from {}: {:?}", addr.to_string(), e),
            }
        }
        warn!("Host: Receiver loop exited.");
    }

    /// The send loop, receive protocol requests and post to crawlers over UDP.
    async fn sender_loop(mut send_socket: SendHalf, mut rx: mpsc::Receiver<(Frame, SocketAddrV4)>) {
        while let Some((frame, peer)) = rx.recv().await {
            let peer = SocketAddr::V4(peer);

            send_socket.send_to(frame.write().as_slice(), &peer).await;
        }
        warn!("Host: Sender loop exited.");
    }

    /// Bind socket and start
    pub async fn start(&mut self) -> Result<()> {
        // Create Udp socket and bind local address.
        let socket = UdpSocket::bind(&self.addr).await?;
        info!("Bind the local socket at udp://{}", self.addr);

        // Create channel to send/recv task
        let (tx, rx) = mpsc::channel::<(Frame, SocketAddrV4)>(100);
        let (recv_socket, send_socket) = socket.split();

        // Spawn send/recv IO tasks.
        tokio::spawn(Self::receiver_loop(
            recv_socket,
            Arc::clone(&self.nodes),
            Arc::clone(&self.wait_queue),
        ));
        tokio::spawn(Self::sender_loop(send_socket, rx));
        self.sender = Some(tx);

        Ok(())
    }

    /// Send a request and return the response.
    pub async fn request(&mut self, body: Body, timeout: u64) -> Result<Body> {
        let node = self.choose_node(&body).await.ok_or(HostError::NoCarawler)?;

        if let Some(sender) = &mut self.sender {
            let (tx, rx) = oneshot::channel::<(Frame, SocketAddrV4)>();
            let frame = Frame::new(body)?;
            let seq = frame.seq;

            // Acquire wait queue, add sender and release lock immediately.
            let mut wait_queue = self.wait_queue.lock().await;
            wait_queue.insert(seq, tx);
            drop(wait_queue);

            info!("Send request to {}: {:?}", node, frame);
            sender
                .send((frame, node))
                .await
                .map_err(|e| TaskError::SendError(format!("Failed to send at request: {:?}", e)))?;

            let response = tokio::time::timeout(Duration::from_millis(timeout), rx).await;
            return match response {
                Ok(Ok((frame, _))) => Ok(frame.body),
                _ => Err(HostError::Timeout.into()),
            };
        }
        return Err(HostError::NoCarawler.into());
    }
}
