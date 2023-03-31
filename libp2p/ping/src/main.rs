// simple ping program based on libp2p ping example

use futures::executor::block_on;
use futures::prelude::*;
use libp2p::swarm::{keep_alive, NetworkBehaviour, Swarm, SwarmBuilder};
use libp2p::{identity, ping, Multiaddr, PeerId};
use std::error::Error;
use std::task::Poll;
use std::time::Duration;

/// Ping network behaviour
#[derive(NetworkBehaviour)]
struct PingBehaviour {
    keep_alive: keep_alive::Behaviour,
    ping: ping::Behaviour,
}

impl PingBehaviour {
    fn new() -> Self {
        PingBehaviour {
            keep_alive: keep_alive::Behaviour::default(),
            ping: ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(1))),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // create key and peer id
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    // create transport
    let transport = block_on(libp2p::development_transport(local_key))?;

    // create a ping network behaviour that pings every seconds
    let behaviour = PingBehaviour::new();

    // create swarm
    let mut swarm =
        SwarmBuilder::with_async_std_executor(transport, behaviour, local_peer_id).build();

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
    let mut listening = false;
    block_on(future::poll_fn(move |cx| loop {
        match swarm.poll_next_unpin(cx) {
            Poll::Ready(Some(event)) => println!("{:?}", event),
            Poll::Ready(None) => return Poll::Ready(()),
            Poll::Pending => {
                if !listening {
                    for addr in Swarm::listeners(&swarm) {
                        println!("Listening on {}", addr);
                        listening = true;
                    }
                }
                return Poll::Pending;
            }
        }
    }));

    Ok(())
}
