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
use std::error::Error;
use std::task::Poll;
use std::time::Duration;
use wasm_timer::Delay;

// floodsub topic
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("/hello/world"));

// ping and pong messages
const PING: &[u8] = "ping".as_bytes();
const PONG: &[u8] = "pong".as_bytes();

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
            let data = String::from_utf8_lossy(&message.data);

            // handle message types "ping" and "pong"
            match data.as_ref() {
                "ping" => {
                    // received ping, send pong
                    self.floodsub.publish(TOPIC.clone(), PONG);
                }
                "pong" => {
                    // received pong
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
    // create key and peer id
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    // create transport
    let transport = block_on(libp2p::development_transport(local_key))?;

    // create floodsub
    let mut floodsub = Floodsub::new(local_peer_id.clone());

    // subscribe to floodsub topic
    floodsub.subscribe(TOPIC.clone());

    // create mdns
    let mdns = block_on(Mdns::new(MdnsConfig::default()))?;

    // create behaviour
    let behaviour = PingBehaviour { floodsub, mdns };

    // create swarm
    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

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
                    swarm.behaviour_mut().floodsub.publish(TOPIC.clone(), PING);

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
