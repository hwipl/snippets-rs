use futures::{executor::block_on, StreamExt};
use libp2p::core::ConnectedPoint;
use libp2p::kad::{
    self, store::MemoryStore, GetRecordOk::FoundRecord, PeerRecord, QueryResult, Quorum, Record,
    RecordKey,
};
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::{Multiaddr, PeerId, StreamProtocol, SwarmBuilder};
use std::error::Error;
use std::str;
use std::time::Duration;

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
async fn handle_events(swarm: &mut Swarm<kad::Behaviour<MemoryStore>>) {
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(event) => match event {
                kad::Event::InboundRequest { .. } => (),
                kad::Event::OutboundQueryProgressed { result, .. } => {
                    // handle query result
                    match result {
                        QueryResult::GetRecord(Ok(FoundRecord(record))) => {
                            handle_peer_records(record);
                            return;
                        }
                        _ => (),
                    }
                }
                kad::Event::RoutingUpdated { .. } => (),
                kad::Event::UnroutablePeer { .. } => (),
                kad::Event::RoutablePeer { .. } => (),
                kad::Event::PendingRoutablePeer { .. } => (),
                kad::Event::ModeChanged { .. } => (),
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
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // create swarm
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_async_std()
        .with_tcp(
            Default::default(),
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )?
        .with_dns()?
        .with_behaviour(|key| {
            // create kademlia behaviour
            let store = MemoryStore::new(key.public().to_peer_id());
            let config = kad::Config::new(StreamProtocol::new("/hello/world/0.1.0"));
            let behaviour = kad::Behaviour::with_config(key.public().to_peer_id(), store, config);

            Ok(behaviour)
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(5)))
        .build();
    println!("Local peer id: {:?}", swarm.local_peer_id());

    // listen on loopback interface and random port.
    swarm.listen_on("/ip6/::1/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

    // set mode to server
    swarm.behaviour_mut().set_mode(Some(kad::Mode::Server));

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
        swarm.behaviour_mut().get_record(RecordKey::new(&KEY));
    } else {
        // store the hello world message in the dht
        let record = Record {
            key: RecordKey::new(&KEY),
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
