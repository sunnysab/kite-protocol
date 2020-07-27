extern crate bincode;
extern crate kite_protocol;

use kite_protocol::agent;
use kite_protocol::agent::Callback;
use kite_protocol::error::Result;
use kite_protocol::services::{self, Body};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::time::Duration;

pub fn on_request(body: Body) -> Result<Body> {
    println!("请求: {:?}", body);
    Ok(body)
}

#[tokio::main]
async fn main() -> Result<()> {
    let callback = Arc::new(on_request as Callback);

    let mut agent = agent::AgentBuilder::new(8910)
        .host("10.2.0.239", 8288)
        .set_callback(callback.clone())
        .build();

    agent.start().await;

    loop {
        tokio::time::delay_for(Duration::from_secs(1)).await;
    }
}
