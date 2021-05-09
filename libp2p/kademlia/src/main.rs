use futures::executor::block_on;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{record::Key, Kademlia, KademliaConfig, KademliaEvent, Quorum, Record};
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId};
use std::error::Error;

const KEY: &str = "hello world";

// handle swarm events
async fn handle_events(swarm: &mut Swarm<Kademlia<MemoryStore>>) {
    loop {
        match swarm.next_event().await {
            SwarmEvent::Behaviour(event) => match event {
                KademliaEvent::QueryResult { .. } => println!("{:?}", event),
                KademliaEvent::RoutingUpdated { .. } => println!("{:?}", event),
                KademliaEvent::UnroutablePeer { .. } => println!("{:?}", event),
                KademliaEvent::RoutablePeer { .. } => println!("{:?}", event),
                KademliaEvent::PendingRoutablePeer { .. } => println!("{:?}", event),
            },
            e @ SwarmEvent::ConnectionEstablished { .. } => println!("{:?}", e),
            e => println!("{:?}", e),
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
    config.set_protocol_name("/hello/world/0.1.0".as_bytes());
    let behaviour = Kademlia::with_config(local_peer_id.clone(), store, config);

    // create swarm
    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

    // listen on loopback interface and random port.
    swarm.listen_on("/ip6/::1/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

    // connect to peer in first command line argument if present
    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial_addr(remote.clone())?;
        println!("Connecting to {}", addr);

        // add address to peer id in second command line argument
        let peer_id = std::env::args().nth(2).ok_or("peer id missing")?;
        let peer_id: PeerId = peer_id.parse()?;
        swarm.behaviour_mut().add_address(&peer_id, remote.clone());

        // request hello world message from the dht
        swarm
            .behaviour_mut()
            .get_record(&Key::new(&KEY), Quorum::One);
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
