#[macro_use]
extern crate log;
extern crate bincode;
extern crate simple_logger;

use kite_protocol::agent;
use kite_protocol::agent::Callback;
use kite_protocol::error::{Result, TaskError};
use kite_protocol::services::{self, Body};
use simple_logger::init_with_level;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::time::Duration;

#[derive(Clone)]
pub struct Database {
    pub s: u32,
}

pub fn on_request(body: Body, p: Database) -> Result<Body> {
    use kite_protocol::services::Heartbeat;

    println!("num = {}", p.s);
    return match body {
        Body::Heartbeat(heartbeat) => Ok(Body::Heartbeat(heartbeat.pong())),
        _ => Err(TaskError::SendError("test".to_string())),
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    init_with_level(log::Level::Info);

    let db = Database { s: 18u32 };
    let callback = Arc::new(on_request as Callback<Database>);

    let mut agent = agent::AgentBuilder::new(String::from("Agent"), 8910)
        .host("10.2.0.239", 8288)
        .set_callback(callback.clone(), db)
        .set_heartbeat_interval(Duration::from_secs(1))
        .build();

    agent.start().await?;

    loop {
        tokio::time::delay_for(Duration::from_secs(1)).await;
    }
}
