extern crate bincode;
extern crate kite_protocol;

use kite_protocol::error::Result;
use kite_protocol::host::Host;
use std::net::SocketAddr;
use tokio::time::Duration;

#[tokio::main]
async fn main() {
    let mut controller = Host::new(8288).await.unwrap();

    controller.start().await;
    let mut i = 0;

    loop {
        println!("{}", i);
        i += 1;
        tokio::time::delay_for(Duration::from_secs(1)).await;
    }
}
