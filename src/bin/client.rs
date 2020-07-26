extern crate bincode;
extern crate kite_protocol;

use kite_protocol::agent;
use kite_protocol::error::Result;
use kite_protocol::services;

use crate::error::Result;
use crate::node::Node;
use crate::protocol::Body;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Node::bind(8080).await?;

    client
        .connect(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8090))
        .await?;

    loop {
        let payload = crate::services::heartbeat::Heartbeat::Ping(Vec::from("Hello world!"));
        let content = Body::Heartbeat(payload);

        client.send(content).await?;
        tokio::time::delay_for(Duration::from_secs(1)).await;
    }
}