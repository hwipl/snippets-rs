use futures::{executor::block_on, StreamExt};
use libp2p::core::ConnectedPoint;
use libp2p::kad::{
    record::store::MemoryStore, record::Key, GetRecordOk::FoundRecord, Kademlia, KademliaConfig,
    KademliaEvent, PeerRecord, QueryResult, Quorum, Record,
};
use libp2p::swarm::{Swarm, SwarmBuilder, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId, StreamProtocol};
use std::error::Error;
use std::str;

const KEY: &str = "hello world";

// handle records
fn handle_peer_records(peer_record: PeerRecord) {
    let key = peer_record.record.key.to_vec();
    let key = str::from_utf8(&key).unwrap();
    let value = str::from_utf8(&peer_record.record.value).unwrap();
    println!(
        "Got record:\n  publisher: {:?},\n  key: {:?},\n  value: {:?}",
        peer_record.record.publisher, key, value
    );
}

// handle swarm events
async fn handle_events(swarm: &mut Swarm<Kademlia<MemoryStore>>) {
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(event) => match event {
                KademliaEvent::InboundRequest { .. } => (),
                KademliaEvent::OutboundQueryProgressed { result, .. } => {
                    // handle query result
                    match result {
                        QueryResult::GetRecord(Ok(FoundRecord(record))) => {
                            handle_peer_records(record);
                            return;
                        }
                        _ => (),
                    }
                }
                KademliaEvent::RoutingUpdated { .. } => (),
                KademliaEvent::UnroutablePeer { .. } => (),
                KademliaEvent::RoutablePeer { .. } => (),
                KademliaEvent::PendingRoutablePeer { .. } => (),
            },
            SwarmEvent::NewListenAddr { address: addr, .. } => println!("Listening on {}", addr),
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
                    // add peer address to kademlia
                    println!("Added address {:?} of peer {:?}", send_back_addr, peer_id);
                    swarm.behaviour_mut().add_address(&peer_id, send_back_addr);
                }
            }
            SwarmEvent::ConnectionClosed {
                peer_id, endpoint, ..
            } => {
                if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
                    // remove peer address from kademlia
                    println!("Removed address {:?} of peer {:?}", send_back_addr, peer_id);
                    swarm
                        .behaviour_mut()
                        .remove_address(&peer_id, &send_back_addr);
                }
            }
            _ => println!(),
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

    // create kademlia behaviour
    let store = MemoryStore::new(local_peer_id.clone());
    let mut config = KademliaConfig::default();
    config.set_protocol_names(vec![StreamProtocol::new("/hello/world/0.1.0")]);
    let behaviour = Kademlia::with_config(local_peer_id.clone(), store, config);

    // create swarm
    let mut swarm =
        SwarmBuilder::with_async_std_executor(transport, behaviour, local_peer_id).build();

    // listen on loopback interface and random port.
    swarm.listen_on("/ip6/::1/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

    // connect to peer in first command line argument if present
    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote.clone())?;
        println!("Connecting to {}", addr);

        // add address to peer id in second command line argument
        let peer_id = std::env::args().nth(2).ok_or("peer id missing")?;
        let peer_id: PeerId = peer_id.parse()?;
        swarm.behaviour_mut().add_address(&peer_id, remote);

        // request hello world message from the dht
        swarm.behaviour_mut().get_record(Key::new(&KEY));
    } else {
        // store the hello world message in the dht
        let record = Record {
            key: Key::new(&KEY),
            value: b"hi".to_vec(),
            publisher: None,
            expires: None,
        };
        swarm
            .behaviour_mut()
            .put_record(record, Quorum::One)
            .expect("Failed to store record locally.");
    }

    // start main loop
    block_on(handle_events(&mut swarm));

    Ok(())
}
