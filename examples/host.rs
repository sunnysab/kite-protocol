#[macro_use]
extern crate log;
extern crate bincode;
extern crate simple_logger;

use kite_protocol::error::Result;
use kite_protocol::host::Host;
use kite_protocol::services::Body;
use kite_protocol::services::Heartbeat;
use simple_logger::init_with_level;
use std::net::SocketAddr;
use std::time::Instant;
use tokio::time::Duration;

#[tokio::main]
async fn main() {
    init_with_level(log::Level::Info);
    let mut host = Host::new("0.0.0.0", 8288).await.unwrap();

    host.start().await;
    tokio::time::delay_for(Duration::from_secs(2)).await;

    let t = Instant::now();
    let request = Body::Heartbeat(Heartbeat::ping(20));
    let response = host.request(request, 5000).await;

    println!("收到 {:?}，用时 {} 秒", response, t.elapsed().as_secs_f32());

    loop {
        tokio::time::delay_for(Duration::from_secs(1)).await;
    }
}
