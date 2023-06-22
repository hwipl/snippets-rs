// simple ping program using mdns and floodsub based on rust-libp2p examples

use futures::executor::block_on;
use futures::prelude::*;
use futures_timer::Delay;
use libp2p::floodsub::{Floodsub, FloodsubEvent, Topic};
use libp2p::mdns;
use libp2p::swarm::{NetworkBehaviour, Swarm, SwarmBuilder, SwarmEvent};
use libp2p::{identity, PeerId};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::task::Poll;
use std::time::Duration;

// key pair and peer id
static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));

// floodsub topic
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("/hello/world"));

// ping message
#[derive(Debug, Serialize, Deserialize)]
struct PingMessage {
    request: u8,
    peer: Vec<u8>,
}

// ping and pong request types
const PING: u8 = 0;
const PONG: u8 = 1;

impl PingMessage {
    // parse ping message
    fn parse(data: &[u8]) -> Result<Self, serde_cbor::Error> {
        serde_cbor::from_slice(data)
    }

    // create ping message bytes
    fn ping() -> Vec<u8> {
        serde_cbor::to_vec(&PingMessage {
            request: PING,
            peer: vec![0],
        })
        .unwrap()
    }

    // create pong message bytes
    fn pong(peer: PeerId) -> Vec<u8> {
        serde_cbor::to_vec(&PingMessage {
            request: PONG,
            peer: peer.to_bytes(),
        })
        .unwrap()
    }
}

// custom network behaviour with floodsub and mdns
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "PingBehaviourEvent")]
struct PingBehaviour {
    floodsub: Floodsub,
    mdns: mdns::async_io::Behaviour,
}

#[derive(Debug)]
enum PingBehaviourEvent {
    Floodsub(FloodsubEvent),
    Mdns(mdns::Event),
}

impl From<FloodsubEvent> for PingBehaviourEvent {
    fn from(event: FloodsubEvent) -> Self {
        PingBehaviourEvent::Floodsub(event)
    }
}

impl From<mdns::Event> for PingBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        PingBehaviourEvent::Mdns(event)
    }
}

/// handle floodsub event
fn handle_floodsub_event(swarm: &mut Swarm<PingBehaviour>, event: FloodsubEvent) {
    if let FloodsubEvent::Message(message) = event {
        // parse ping message
        let msg = match PingMessage::parse(&message.data) {
            Ok(ping) => ping,
            Err(_) => return,
        };

        // handle ping message types "ping" and "pong"
        match msg.request {
            PING => {
                // received ping, send pong
                swarm
                    .behaviour_mut()
                    .floodsub
                    .publish(TOPIC.clone(), PingMessage::pong(message.source));
            }
            PONG => {
                // received pong, check if it is for us
                let peer = match PeerId::from_bytes(&msg.peer) {
                    Ok(peer) => peer,
                    Err(_) => return,
                };
                if peer != PEER_ID.clone() {
                    return;
                }
                println!("Received ping reply from {:?}", message.source);
            }
            _ => {
                // handle unknown messages
                println!("Received unknown message from {:?}", message.source);
            }
        }
    }
}

/// handle mdns event
fn handle_mdns_event(swarm: &mut Swarm<PingBehaviour>, event: mdns::Event) {
    match event {
        mdns::Event::Discovered(list) => {
            for (peer, _) in list {
                swarm
                    .behaviour_mut()
                    .floodsub
                    .add_node_to_partial_view(peer);
            }
        }
        mdns::Event::Expired(list) => {
            for (peer, _) in list {
                if !swarm.behaviour().mdns.has_node(&peer) {
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .remove_node_from_partial_view(&peer);
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // show peer id
    println!("Local node: {:?}", PEER_ID.clone());

    // create transport
    let transport = block_on(libp2p::development_transport(KEYS.clone()))?;

    // create floodsub
    let mut floodsub = Floodsub::new(PEER_ID.clone());

    // subscribe to floodsub topic
    floodsub.subscribe(TOPIC.clone());

    // create mdns
    let mdns = mdns::Behaviour::new(mdns::Config::default(), PEER_ID.clone())?;

    // create behaviour
    let behaviour = PingBehaviour { floodsub, mdns };

    // create swarm
    let mut swarm =
        SwarmBuilder::with_async_std_executor(transport, behaviour, PEER_ID.clone()).build();

    // listen on all ipv4 and ipv6 addresses and random port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    swarm.listen_on("/ip6/::/tcp/0".parse()?)?;

    // start main loop
    let mut timer = Delay::new(Duration::new(5, 0));
    block_on(future::poll_fn(move |cx| {
        loop {
            // handle swarm events
            let mut swarm_pending = false;
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => {
                    match event {
                        // Floodsub event
                        SwarmEvent::Behaviour(PingBehaviourEvent::Floodsub(event)) => {
                            handle_floodsub_event(&mut swarm, event);
                        }

                        // Mdns event
                        SwarmEvent::Behaviour(PingBehaviourEvent::Mdns(event)) => {
                            handle_mdns_event(&mut swarm, event)
                        }

                        event => println!("{:?}", event),
                    }
                }
                Poll::Ready(None) => {
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => {
                    swarm_pending = true;
                }
            }

            // handle timer events
            match timer.poll_unpin(cx) {
                Poll::Pending => {
                    if swarm_pending {
                        return Poll::Pending;
                    }
                }
                Poll::Ready(()) => {
                    // publish message
                    println!("Sending ping");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(TOPIC.clone(), PingMessage::ping());

                    // reset timer
                    timer.reset(Duration::new(5, 0));
                }
            }
        }
    }))
}
