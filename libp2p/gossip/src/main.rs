use futures::executor::block_on;
use futures::prelude::*;
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::Multiaddr;
use libp2p::{gossipsub, SwarmBuilder};
use std::error::Error;
use std::task::Poll;

fn main() -> Result<(), Box<dyn Error>> {
    // create swarm
    let builder = block_on(
        SwarmBuilder::with_new_identity()
            .with_async_std()
            .with_tcp(
                Default::default(),
                (libp2p::tls::Config::new, libp2p::noise::Config::new),
                libp2p::yamux::Config::default,
            )?
            .with_dns(),
    )?;
    let mut swarm = builder
        .with_behaviour(|key| {
            // create gossipsub behaviour
            let message_authenticity = gossipsub::MessageAuthenticity::Signed(key.clone());
            let gossipsub_config = gossipsub::Config::default();
            let behaviour: gossipsub::Behaviour =
                gossipsub::Behaviour::new(message_authenticity, gossipsub_config)?;
            Ok(behaviour)
        })?
        .build();
    println!("Local peer id: {:?}", swarm.local_peer_id());

    // subscribe to topic
    let topic = gossipsub::IdentTopic::new("/hello/world");
    swarm.behaviour_mut().subscribe(&topic).unwrap();

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
