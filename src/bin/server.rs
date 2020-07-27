extern crate bincode;
extern crate kite_protocol;

use kite_protocol::error::Result;
use kite_protocol::host::Host;
use kite_protocol::services::Body;
use kite_protocol::services::Heartbeat;
use std::net::SocketAddr;
use tokio::time::Duration;

#[tokio::main]
async fn main() {
    let mut host = Host::new(8288).await.unwrap();

    host.start().await;

    tokio::time::delay_for(Duration::from_secs(2)).await;

    let request = Body::Heartbeat(Heartbeat::ping("Hello world"));
    println!("请求 {:?}", request);
    let response = host.request(request, 5000).await;
    println!("响应 {:?}", response);

    loop {
        tokio::time::delay_for(Duration::from_secs(1)).await;
    }
}
