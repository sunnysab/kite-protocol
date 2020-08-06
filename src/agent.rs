use crate::error::{Result, TaskError};
use crate::protocol::Frame;
use crate::services::{AgentBasic, Body};
use log::{debug, error, info, warn};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::udp::{RecvHalf, SendHalf};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::Duration;

pub type Callback<T> = fn(Body, T) -> Result<Body>;

/// Default heartbeat interval in sec.
const DEFAULT_HEARTBEAT_INTERVAL_SEC: u32 = 60;

/// Agent Builder, in builder pattern.
pub struct AgentBuilder<T> {
    /// Agent name
    name: String,
    /// Agent local address string
    local_addr: String,
    /// Host address.
    host_addr: Option<SocketAddrV4>,
    /// Callback function will be called when host requests
    request_callback: Option<Arc<Callback<T>>>,
    /// Callback parameter.
    parameter: Option<T>,
    /// Heartbeat interval
    heartbeat_interval: Option<Duration>,
}

impl<T> AgentBuilder<T>
where
    T: 'static + std::marker::Send + Clone,
{
    /// Create and initialize agent(proxy) node.
    pub fn new(local_name: String, local_port: u16) -> Self {
        Self {
            name: local_name,
            local_addr: format!("0.0.0.0:{}", local_port),
            host_addr: None,
            request_callback: None,
            parameter: None,
            heartbeat_interval: None,
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
    pub fn set_callback(mut self, callback_fn: Arc<Callback<T>>, parameter: T) -> Self {
        self.request_callback = Some(callback_fn.clone());
        self.parameter = Some(parameter);
        self
    }

    /// Set heartbeat interval
    pub fn set_heartbeat_interval(mut self, duration: Duration) -> Self {
        self.heartbeat_interval = Some(duration);
        self
    }

    /// Build a valid Agent structure. `panic` if host or callback function is not set.
    pub fn build(self) -> Agent<T> {
        Agent {
            name: self.name,
            local_addr: self.local_addr,
            host_addr: self.host_addr.expect("Host address is needed."),
            request_callback: self
                .request_callback
                .expect("You should set callback function.")
                .clone(),
            parameter: self.parameter.expect("You should set callback parameter."),
            heartbeat_inerval: self
                .heartbeat_interval
                .unwrap_or(Duration::from_secs(DEFAULT_HEARTBEAT_INTERVAL_SEC as u64)),
        }
    }
}

/// Agent node in campus side.
pub struct Agent<T> {
    name: String,
    /// Agent local address string
    local_addr: String,
    /// Host address.
    host_addr: SocketAddrV4,
    /// Callback function will be called when host requests
    request_callback: Arc<Callback<T>>,
    /// Callback parameter
    parameter: T,
    /// Default heartbeat interval
    heartbeat_inerval: Duration,
}

impl<T> Agent<T>
where
    T: 'static + std::marker::Send + Clone,
{
    /// Receive packets from the inner channel and transfer them to host over Udp.
    /// It will be run as a tokio task, and exites when tx closed.
    // TODO: Support close this task manually.
    async fn sender_loop(
        host_addr: SocketAddrV4,
        mut send_socket: SendHalf,
        mut rx: mpsc::Receiver<Frame>,
    ) {
        // Convert SocketAddrV4 to SocketAddr
        let host_addr = SocketAddr::V4(host_addr);

        while let Some(mut frame) = rx.recv().await {
            let r = send_socket
                .send_to(frame.write().as_slice(), &host_addr)
                .await;
            match r {
                Ok(size) => info!("Send a packet with size = {}", size),
                Err(e) => error!("Agent: Failed to send frame: {}", e),
            }
        }
        warn!("Agent: sender loop exited.");
    }

    /// Preset heartbeat loop. When the network status changes, the heartbeat packet can prompt
    /// the host to discover the new address of the Agent.
    async fn heartbeat_loop(mut tx: mpsc::Sender<Frame>, duration: Duration, name: String) {
        loop {
            let agent_basic = AgentBasic {
                name: name.clone(),
                local_addr: "".to_string(),
            };

            if let Ok(frame) = Frame::new_request(Body::Discovery(agent_basic)) {
                if let Err(e) = tx.send(frame).await {
                    error!("Failed to send a heartbeat frame to sender loop: {}", e);
                }
            } else {
                error!("Failed to make heartbeat frame.");
            }
            debug!("Send a heartbeat frame just now.");
            // Pause for a few seconds
            tokio::time::delay_for(duration).await;
        }
    }

    /// Run callback function to process requests, and send calculation result to sender loop.
    async fn process(
        request: Frame,              // Request frame.
        mut tx: mpsc::Sender<Frame>, // Channel to sender loop.
        imcoming: Arc<Callback<T>>,  // Callback function.
        callback_data: T,            // Callback parameter defined by user.
    ) -> Result<()> {
        let response_ack = request.seq;

        // Call the callback function in a separated thread.
        let result =
            tokio::task::spawn_blocking(move || imcoming(request.body, callback_data)).await?;
        // Make response frame with request seq and response body.
        let frame = Frame::new_response(result?, response_ack)?;
        // Send to sender loop.
        tx.send(frame)
            .await
            .map_err(|e| TaskError::SendError(e.to_string()))?;
        Ok(())
    }

    /// Receiver loop, accept commands and requests from the host and execute them.
    async fn receiver_loop(
        mut recv_socket: RecvHalf,  // Udp recv half.
        tx: mpsc::Sender<Frame>,    // Channel to sender loop.
        imcoming: Arc<Callback<T>>, // Callback function.
        parameter: T,
    ) {
        // Alloc a 512K size buffer.
        let mut buffer = vec![0u8; 512 * 1024];

        info!("Start listening...");
        loop {
            let r = recv_socket.recv_from(&mut buffer).await;

            if let Ok((size, addr)) = r {
                // Note:
                // If agent send heartbeat packet to host, while the host is offline, the operating system under host may
                // return an "unreachable" notice which has a zero size. We should ignore them, or many warnings occur.
                if size == 0 {
                    continue;
                }
                // Unpack and get the request frame
                match Frame::read(&mut buffer[..size]) {
                    Ok(frame) => {
                        // Create a coroutine to process.
                        debug!("Process packet from {:?}, size = {}", addr, size);
                        tokio::spawn(Self::process(
                            frame,
                            tx.clone(),
                            imcoming.clone(),
                            parameter.clone(),
                        ));
                    }
                    Err(e) => warn!("Failed to unpack frame from {}: {:?}", addr.to_string(), e),
                }
            } else {
                warn!("In recv_loop, recv_from throws {:?}", r);
                break;
            }
        }
        warn!("Receiver loop exited.");
    }

    /// Bind a local Udp socket and setup sender, receiver and heartbeat loop.
    /// # Examples
    /// ```rust
    /// let callback = Arc::new(on_request as Callback);
    ///
    /// let mut agent = agent::AgentBuilder::new(8910)
    ///     .host("10.2.0.239", 8288)
    ///     .set_callback(callback.clone())
    ///     .build();
    ///
    /// agent.start().await;
    /// ```
    pub async fn start(&mut self) -> Result<()> {
        let socket = UdpSocket::bind(&self.local_addr).await?;
        info!("Bind the local socket at udp://{}", self.local_addr);

        let (recv_socket, send_socket) = socket.split();
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(Self::sender_loop(self.host_addr, send_socket, rx));
        tokio::spawn(Self::receiver_loop(
            recv_socket,
            tx.clone(),
            Arc::clone(&self.request_callback),
            self.parameter.clone(),
        ));
        tokio::spawn(Self::heartbeat_loop(
            tx,
            self.heartbeat_inerval,
            self.name.clone(),
        ));
        Ok(())
    }
}
