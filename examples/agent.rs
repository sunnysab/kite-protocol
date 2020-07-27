#[macro_use]
extern crate log;
extern crate bincode;
extern crate simple_logger;

use kite_protocol::agent;
use kite_protocol::agent::Callback;
use kite_protocol::error::Result;
use kite_protocol::services::{self, Body};
use simple_logger::init_with_level;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::time::Duration;

pub fn on_request(body: Body) -> Result<Body> {
    println!("请求: {:?}", body);
    Ok(body)
}

#[tokio::main]
async fn main() -> Result<()> {
    init_with_level(log::Level::Info);

    let callback = Arc::new(on_request as Callback);

    let mut agent = agent::AgentBuilder::new(8910)
        .host("10.2.0.239", 8288)
        .set_callback(callback.clone())
        .set_heartbeart_interval(Duration::from_secs(1))
        .build();

    agent.start().await?;

    loop {
        tokio::time::delay_for(Duration::from_secs(1)).await;
    }
}
