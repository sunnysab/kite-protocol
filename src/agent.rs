use crate::error::Result;
use crate::protocol::Frame;
use crate::services::{Body, Heartbeat};
use std::borrow::Borrow;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::udp::{RecvHalf, SendHalf};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::Duration;

pub type Callback = fn(Body) -> Result<Body>;

/// Agent Builder, in builder pattern.
pub struct AgentBuilder {
    /// Agent local address string
    local_addr: String,
    /// Host address.
    host_addr: Option<SocketAddrV4>,
    /// Callback function will be called when host requests
    request_callback: Option<Arc<Callback>>,
}

impl AgentBuilder {
    /// Create and initialize agent(proxy) node.
    pub fn new(local_port: u16) -> Self {
        Self {
            local_addr: format!("0.0.0.0:{}", local_port),
            host_addr: None,
            request_callback: None,
        }
    }

    /// Set host address
    pub fn host(mut self, addr: &str, port: u16) -> Self {
        self.host_addr = Some(SocketAddrV4::new(
            Ipv4Addr::from_str(addr).expect("Host address must be a valid IPv4 address."),
            port,
        ));
        self
    }

    /// Set callback function which will be called when packet comes.
    pub fn set_callback(mut self, callback_fn: Arc<Callback>) -> Self {
        self.request_callback = Some(callback_fn.clone());
        self
    }

    pub fn build(self) -> Agent {
        Agent {
            local_addr: self.local_addr,
            host_addr: self.host_addr.expect("Host address is needed."),
            request_callback: self
                .request_callback
                .expect("You should set callback function.")
                .clone(),
        }
    }
}

pub struct Agent {
    /// Agent local address string
    local_addr: String,
    /// Host address.
    host_addr: SocketAddrV4,
    /// Callback function will be called when host requests
    request_callback: Arc<Callback>,
}

impl Agent {
    async fn send_loop(
        host_addr: SocketAddrV4,
        mut send_socket: SendHalf,
        mut rx: mpsc::Receiver<Frame>,
    ) {
        let host_addr = SocketAddr::V4(host_addr);

        while let Some(frame) = rx.recv().await {
            send_socket
                .send_to(frame.write().as_slice(), &host_addr)
                .await;
        }
    }
    async fn heartbeat_loop(mut tx: mpsc::Sender<Frame>) {
        loop {
            let heartbeat_request = Heartbeat::ping("Hello world");
            let frame = Frame::new(Body::Heartbeat(heartbeat_request)).unwrap();

            tx.send(frame).await;
            tokio::time::delay_for(Duration::from_secs(1)).await;
        }
    }

    async fn process(
        request: Frame,
        mut tx: mpsc::Sender<Frame>,
        imcoming: Arc<Callback>,
    ) -> Result<()> {
        let request_seq = request.seq;

        let result: Result<Body> =
            tokio::task::spawn_blocking(move || imcoming(request.body)).await?;
        let frame = Frame::new_response(result.unwrap(), request_seq)?;

        println!("返回 {:?}", frame);
        tx.send(frame).await;
        Ok(())
    }

    async fn recv_loop(
        mut recv_socket: RecvHalf,
        tx: mpsc::Sender<Frame>,
        imcoming: Arc<Callback>,
    ) {
        let mut buffer = vec![0u8; 512 * 1024];

        while let Ok((size, _)) = recv_socket.recv_from(&mut buffer).await {
            match Frame::read(&mut buffer[..size]) {
                Ok(frame) => {
                    tokio::spawn(Self::process(frame, tx.clone(), imcoming.clone()));
                }
                Err(_) => (),
            }
        }
    }
    pub async fn start(&mut self) -> Result<()> {
        let socket = UdpSocket::bind(&self.local_addr).await?;

        let (recv_socket, send_socket) = socket.split();
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(Self::send_loop(self.host_addr, send_socket, rx));
        tokio::spawn(Self::recv_loop(
            recv_socket,
            tx.clone(),
            Arc::clone(&self.request_callback),
        ));
        tokio::spawn(Self::heartbeat_loop(tx));
        Ok(())
    }
}
