use crate::error::{Result, TaskError};
use crate::node::Node;
use crate::protocol::{Body, Frame, PACK_REQUEST};
use chrono::Utc;
use std::borrow::BorrowMut;
use std::cell::Cell;
use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::udp::{RecvHalf, SendHalf};
use tokio::net::UdpSocket;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::Duration;

pub type SeqType = u32;
pub type SegType = u8;

pub enum ControllerError {}

/// Controller, as server side node
pub struct Controller {
    /// Crawler nodes.
    nodes: Vec<Node>,
    /// Local address and port
    addr: String,

    sender: Option<Sender<(Frame, SocketAddrV4)>>,

    wait_queue: Arc<Mutex<HashMap<SeqType, mpsc::Sender<(Frame, SocketAddrV4)>>>>,

    /* Statisic information. */
    /// Count of sent frames.
    pub frame_sent: usize,
    /// Count of received frames.
    pub frame_recv: usize,
}

impl Controller {
    /// Create a controller node.
    pub async fn new(port: u16) -> Result<Self> {
        let local_addr = format!("0.0.0.0:{}", port);

        Ok(Self {
            nodes: vec![],
            addr: local_addr,
            sender: None,
            // receiver: None,
            wait_queue: Arc::new(Mutex::new(HashMap::new())),
            frame_sent: 0,
            frame_recv: 0,
        })
    }

    async fn receive_loop(
        mut recv_socket: RecvHalf,
        wait_queue: Arc<Mutex<HashMap<SeqType, mpsc::Sender<(Frame, SocketAddrV4)>>>>,
    ) {
        let mut buffer = vec![0u8; 512 * 1024];

        loop {
            match recv_socket.recv_from(&mut buffer).await {
                Ok((size, SocketAddr::V4(addr))) => {
                    let frame = Frame::read(&mut buffer[..size]).unwrap();
                    let seq = frame.seq;
                    let mut queue = wait_queue.lock().await;

                    // TODO: Use oneshot instead of mpsc.
                    if let Some(target) = queue.get_mut(&seq) {
                        target.send((frame, addr));
                        queue.remove(&seq);
                    }
                }
                _ => {}
            }
        }
    }

    async fn send_loop(mut send_socket: SendHalf, mut rx: Receiver<(Frame, SocketAddrV4)>) {
        while let Some((frame, peer)) = rx.recv().await {
            let peer = SocketAddr::V4(peer);

            send_socket.send_to(frame.write().as_slice(), &peer).await;
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        // Create channel to send/recv task
        let (tx, rx) = mpsc::channel::<(Frame, SocketAddrV4)>(100);

        // Create Udp socket and bind local address.
        let socket = UdpSocket::bind(&self.addr).await?;
        let (recv_socket, send_socket) = socket.split();

        // Spawn send/recv IO tasks.
        tokio::spawn(Self::receive_loop(
            recv_socket,
            Arc::clone(&self.wait_queue),
        ));
        tokio::spawn(Self::send_loop(send_socket, rx));
        self.sender = Some(tx);

        Ok(())
    }

    fn choose_node(&mut self, _body: &Body) -> Option<SocketAddrV4> {
        if self.nodes.len() != 0 {
            // TODO: Choose recently used.
            // random_choose
            self.nodes[0].last_update = Utc::now().naive_local();
            return Some(self.nodes[0].node_addr);
        }
        return None;
    }

    pub async fn request(&mut self, body: Body, timeout: u64) -> Result<Frame> {
        if self.sender.is_none() {
            return Err(TaskError::ControllerError);
        }

        let node = self.choose_node(&body).unwrap();
        if let Some(sender) = &mut self.sender {
            let (mut tx, mut rx) = mpsc::channel::<(Frame, SocketAddrV4)>(1);
            let frame = Frame::new(body, PACK_REQUEST).unwrap();
            let seq = frame.seq;

            sender.send((frame, node)).await;

            let mut wait_queue = self.wait_queue.lock().await;
            wait_queue.insert(seq, tx);
            drop(wait_queue);

            let response = tokio::time::timeout(Duration::from_millis(timeout), rx.recv()).await;
            return match response {
                Ok(Some((frame, _))) => Ok(frame),
                _ => Err(TaskError::ControllerError),
            };
        }
        return Err(TaskError::ControllerError);
    }
}
