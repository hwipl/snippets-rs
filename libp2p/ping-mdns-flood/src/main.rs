// simple ping program using mdns and floodsub based on rust-libp2p examples

use futures::executor::block_on;
use futures::prelude::*;
use libp2p::floodsub::{Floodsub, FloodsubEvent, Topic};
use libp2p::mdns::{Mdns, MdnsConfig, MdnsEvent};
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::swarm::Swarm;
use libp2p::NetworkBehaviour;
use libp2p::{identity, PeerId};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::task::Poll;
use std::time::Duration;
use wasm_timer::Delay;

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
struct PingBehaviour {
    floodsub: Floodsub,
    mdns: Mdns,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for PingBehaviour {
    // Called when `floodsub` produces an event.
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            // parse ping message
            let msg = match PingMessage::parse(&message.data) {
                Ok(ping) => ping,
                Err(_) => return,
            };

            // handle ping message types "ping" and "pong"
            match msg.request {
                PING => {
                    // received ping, send pong
                    self.floodsub
                        .publish(TOPIC.clone(), PingMessage::pong(message.source));
                }
                PONG => {
                    // received pong, check if it is for us
                    let peer = PeerId::from_bytes(&msg.peer).unwrap();
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
}

impl NetworkBehaviourEventProcess<MdnsEvent> for PingBehaviour {
    // Called when `mdns` produces an event.
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self.floodsub.add_node_to_partial_view(peer);
                    println!("mdns: added {:?} to floodsub", peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                        println!("mdns: removed {:?} from floodsub", peer)
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // show peer id
    println!("Local peer id: {:?}", PEER_ID);

    // create transport
    let transport = block_on(libp2p::development_transport(KEYS.clone()))?;

    // create floodsub
    let mut floodsub = Floodsub::new(PEER_ID.clone());

    // subscribe to floodsub topic
    floodsub.subscribe(TOPIC.clone());

    // create mdns
    let mdns = block_on(Mdns::new(MdnsConfig::default()))?;

    // create behaviour
    let behaviour = PingBehaviour { floodsub, mdns };

    // create swarm
    let mut swarm = Swarm::new(transport, behaviour, PEER_ID.clone());

    // listen on all ipv4 and ipv6 addresses and random port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    swarm.listen_on("/ip6/::/tcp/0".parse()?)?;

    // start main loop
    let mut timer = Delay::new(Duration::new(5, 0));
    let mut listening = false;
    block_on(future::poll_fn(move |cx| {
        loop {
            // handle swarm events
            let mut swarm_pending = false;
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => println!("{:?}", event),
                Poll::Ready(None) => {
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => {
                    if !listening {
                        for addr in Swarm::listeners(&swarm) {
                            println!("Listening on {:?}", addr);
                            listening = true;
                        }
                    }
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
                Poll::Ready(Ok(())) => {
                    // publish message
                    println!("Sending ping");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(TOPIC.clone(), PingMessage::ping());

                    // reset timer
                    timer.reset(Duration::new(5, 0));
                }
                Poll::Ready(Err(_)) => {
                    panic!("timer error");
                }
            }
        }
    }))
}
