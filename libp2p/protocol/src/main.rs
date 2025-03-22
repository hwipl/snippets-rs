// simple ping program based on libp2p ping example

mod protocol;

use futures::StreamExt;
use libp2p::{swarm::SwarmEvent, Multiaddr, SwarmBuilder};
use std::error::Error;

use protocol::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // create swarm
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            Default::default(),
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )?
        .with_dns()?
        .with_behaviour(|_key| {
            // create a hello world network behaviour that sends hello world messages
            HelloWorld::new()
        })?
        .build();
    println!("Local peer id: {:?}", swarm.local_peer_id());

    // listen on loopback interface and random port.
    swarm.listen_on("/ip6/::1/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

    // connect to peer in first command line argument if present
    if let Some(addr) = std::env::args().nth(1) {
        let remote = addr.parse::<Multiaddr>()?;
        swarm.dial(remote)?;
        println!("Connecting to {}", addr)
    }

    // start main loop
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {address}")
            }
            event => println!("{:?}", event),
        }
    }
}
