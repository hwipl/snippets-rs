use futures::executor::block_on;
use futures::prelude::*;
use libp2p::gossipsub;
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId};
use std::error::Error;
use std::task::Poll;

fn main() -> Result<(), Box<dyn Error>> {
    // create key and peer id
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    // create transport
    let transport = block_on(libp2p::development_transport(local_key.clone()))?;

    // create gossipsub behaviour
    let message_authenticity = gossipsub::MessageAuthenticity::Signed(local_key);
    let gossipsub_config = gossipsub::Config::default();
    let mut behaviour: gossipsub::Behaviour =
        gossipsub::Behaviour::new(message_authenticity, gossipsub_config)?;

    // subscribe to topic
    let topic = gossipsub::IdentTopic::new("/hello/world");
    behaviour.subscribe(&topic).unwrap();

    // create swarm
    let mut swarm = Swarm::with_async_std_executor(transport, behaviour, local_peer_id);

    // listen on loopback interface and random port.
    swarm.listen_on("/ip6/::1/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

    // connect to peer in first command line argument if present
    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        println!("Connecting to {}", addr)
    }

    // start main loop
    let mut listening = false;
    block_on(future::poll_fn(move |cx| loop {
        match swarm.poll_next_unpin(cx) {
            Poll::Ready(Some(event)) => match event {
                SwarmEvent::Behaviour(gossipsub::Event::Message { message, .. }) => {
                    println!(
                        "Got message from {:?} to {:?}: {:?}",
                        message.source,
                        message.topic,
                        String::from_utf8_lossy(&message.data)
                    );
                }
                SwarmEvent::Behaviour(gossipsub::Event::Subscribed {
                    peer_id: _peer_id,
                    topic: _t,
                }) => {
                    // println!("Subscribed: {:?} {:?}", peer_id, t);
                    // swarm.behaviour_mut().add_explicit_peer(&peer_id);
                    swarm
                        .behaviour_mut()
                        .publish(topic.clone(), b"hi".to_vec())
                        .unwrap();
                }
                SwarmEvent::Behaviour(gossipsub::Event::Unsubscribed { .. }) => {
                    // println!("Unsubscribed");
                }
                _ => (),
            },
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
