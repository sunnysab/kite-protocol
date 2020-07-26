extern crate bincode;

mod controller;
mod error;
mod modules;
mod node;
mod protocol;

use crate::controller::Controller;
use crate::error::Result;
use std::net::SocketAddr;
use tokio::time::Duration;

async fn on_message(buffer: &[u8], node_addr: SocketAddr) {
    println!("{:?} -> {:?}", node_addr, buffer);
}

#[tokio::main]
async fn main() {
    let mut controller = Controller::new(8288).await.unwrap();

    controller.start().await;
    let mut i = 0;

    loop {
        println!("{}", i);
        i += 1;
        tokio::time::delay_for(Duration::from_secs(1)).await;
    }
}
