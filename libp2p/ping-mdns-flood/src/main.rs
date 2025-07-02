// simple ping program using mdns and floodsub based on rust-libp2p examples

use futures::StreamExt;
use libp2p::swarm::{NetworkBehaviour, Swarm, SwarmEvent};
use libp2p::{floodsub, mdns, PeerId, SwarmBuilder};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::LazyLock;
use tokio::select;
use tokio::time::{self, Duration, Instant};

// floodsub topic
static TOPIC: LazyLock<floodsub::Topic> = LazyLock::new(|| floodsub::Topic::new("/hello/world"));

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
    floodsub: floodsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
enum PingBehaviourEvent {
    Floodsub(floodsub::Event),
    Mdns(mdns::Event),
}

impl From<floodsub::Event> for PingBehaviourEvent {
    fn from(event: floodsub::Event) -> Self {
        PingBehaviourEvent::Floodsub(event)
    }
}

impl From<mdns::Event> for PingBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        PingBehaviourEvent::Mdns(event)
    }
}

/// handle floodsub event
fn handle_floodsub_event(swarm: &mut Swarm<PingBehaviour>, event: floodsub::Event) {
    if let floodsub::Event::Message(message) = event {
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
                if peer != swarm.local_peer_id().clone() {
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
                if !swarm
                    .behaviour()
                    .mdns
                    .discovered_nodes()
                    .any(|x| x == &peer)
                {
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .remove_node_from_partial_view(&peer);
                }
            }
        }
    }
}

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
        .with_behaviour(|key| {
            // create floodsub
            let mut floodsub = floodsub::Behaviour::new(key.public().to_peer_id());

            // subscribe to floodsub topic
            floodsub.subscribe(TOPIC.clone());

            // create mdns
            let mdns = mdns::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;

            // create behaviour
            Ok(PingBehaviour { floodsub, mdns })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(5)))
        .build();
    println!("Local peer id: {:?}", swarm.local_peer_id());

    // listen on all ipv4 and ipv6 addresses and random port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    swarm.listen_on("/ip6/::/tcp/0".parse()?)?;

    // start main loop
    let timer = time::sleep(Duration::new(5, 0));
    tokio::pin!(timer);
    loop {
        select! {
            // handle timer events
            _ = &mut timer => {
                // publish message
                println!("Sending ping");
                swarm
                    .behaviour_mut()
                    .floodsub
                    .publish(TOPIC.clone(), PingMessage::ping());

                // reset timer
                timer.as_mut().reset(Instant::now() + Duration::new(5,0));
            }
            // handle swarm events
            event = swarm.select_next_some() => match event {
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
    }
}
