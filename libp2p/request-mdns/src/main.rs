// simple request response program with mdns discovery

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::mdns;
use libp2p::request_response;
use libp2p::swarm::{NetworkBehaviour, Swarm, SwarmEvent};
use libp2p::{PeerId, SwarmBuilder};
use std::error::Error;
use std::time::Duration;
use std::{io, iter};
use tracing_subscriber::EnvFilter;

/// Hello protocol for the request response behaviour
#[derive(Debug, Clone)]
struct HelloProtocol();

impl AsRef<str> for HelloProtocol {
    fn as_ref(&self) -> &str {
        "/hello/0.0.1"
    }
}

/// Hello codec for the request response behaviour
#[derive(Clone, Default)]
struct HelloCodec();

#[async_trait]
impl request_response::Codec for HelloCodec {
    type Protocol = HelloProtocol;
    type Request = HelloRequest;
    type Response = HelloResponse;

    async fn read_request<T>(&mut self, _: &HelloProtocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // TODO: add length
        let mut vec = Vec::new();
        io.take(1024).read_to_end(&mut vec).await?;
        if vec.is_empty() {
            // println!("1e");
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        Ok(HelloRequest(vec))
    }

    async fn read_response<T>(
        &mut self,
        _: &HelloProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();
        io.take(1024).read_to_end(&mut vec).await?;
        if vec.is_empty() {
            // println!("2e");
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        Ok(HelloResponse(vec))
    }

    async fn write_request<T>(
        &mut self,
        _: &HelloProtocol,
        io: &mut T,
        HelloRequest(data): HelloRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        io.write_all(data.as_ref()).await
    }

    async fn write_response<T>(
        &mut self,
        _: &HelloProtocol,
        io: &mut T,
        HelloResponse(data): HelloResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        io.write_all(data.as_ref()).await
    }
}

/// Request message
#[derive(Debug, Clone, PartialEq, Eq)]
struct HelloRequest(Vec<u8>);

/// Response message
#[derive(Debug, Clone, PartialEq, Eq)]
struct HelloResponse(Vec<u8>);

/// Custom network behaviour with request response and mdns
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "HelloBehaviourEvent")]
struct HelloBehaviour {
    request: request_response::Behaviour<HelloCodec>,
    mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
enum HelloBehaviourEvent {
    RequestResponse(request_response::Event<HelloRequest, HelloResponse>),
    Mdns(mdns::Event),
}

impl From<request_response::Event<HelloRequest, HelloResponse>> for HelloBehaviourEvent {
    fn from(event: request_response::Event<HelloRequest, HelloResponse>) -> Self {
        HelloBehaviourEvent::RequestResponse(event)
    }
}

impl From<mdns::Event> for HelloBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        HelloBehaviourEvent::Mdns(event)
    }
}

/// handle RequestResponse event
fn handle_request_response_event(
    swarm: &mut Swarm<HelloBehaviour>,
    event: request_response::Event<HelloRequest, HelloResponse>,
) {
    // create messages
    let request = HelloRequest("hey".to_string().into_bytes());
    let response = HelloResponse("hi".to_string().into_bytes());

    // handle incoming messages
    if let request_response::Event::Message {
        peer,
        connection_id: _,
        message,
    } = event
    {
        match message {
            // handle incoming request message, send back response
            request_response::Message::Request { channel, .. } => {
                println!("received request {:?} from {:?}", request, peer);
                swarm
                    .behaviour_mut()
                    .request
                    .send_response(channel, response.clone())
                    .unwrap();
                return;
            }

            // handle incoming response message
            request_response::Message::Response { response, .. } => {
                println!("received response {:?} from {:?}", response, peer);
                return;
            }
        }
    }

    // handle response sent event
    if let request_response::Event::ResponseSent { peer, .. } = event {
        println!("sent response {:?} to {:?}", response, peer);
        return;
    }

    println!("request response error: {:?}", event);
}

/// handle Mdns event
fn handle_mdns_event(swarm: &mut Swarm<HelloBehaviour>, event: mdns::Event) {
    let request = HelloRequest("hey".to_string().into_bytes());
    match event {
        mdns::Event::Discovered(list) => {
            let mut new_peers: Vec<PeerId> = Vec::new();
            for (peer, addr) in list {
                swarm.add_peer_address(peer, addr.clone());
                if new_peers.contains(&peer) {
                    continue;
                }
                new_peers.push(peer);
            }

            for peer in new_peers {
                swarm
                    .behaviour_mut()
                    .request
                    .send_request(&peer, request.clone());
            }
        }
        mdns::Event::Expired(list) => {
            for (peer, addr) in list {
                if !swarm
                    .behaviour()
                    .mdns
                    .discovered_nodes()
                    .any(|x| x == &peer)
                {
                    swarm.behaviour_mut().request.remove_address(&peer, &addr);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

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
            // create mdns
            let mdns = mdns::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;

            // create request response
            let protocols = iter::once((HelloProtocol(), request_response::ProtocolSupport::Full));
            let cfg = request_response::Config::default();
            let request = request_response::Behaviour::new(protocols.clone(), cfg.clone());

            // create network behaviour
            Ok(HelloBehaviour { request, mdns })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(10)))
        .build();
    println!("Local peer id: {:?}", swarm.local_peer_id());

    // listen on all addresses and random port.
    swarm.listen_on("/ip6/::/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // start main loop
    loop {
        match swarm.select_next_some().await {
            // RequestResponse event
            SwarmEvent::Behaviour(HelloBehaviourEvent::RequestResponse(event)) => {
                handle_request_response_event(&mut swarm, event);
            }

            // Mdns event
            SwarmEvent::Behaviour(HelloBehaviourEvent::Mdns(event)) => {
                handle_mdns_event(&mut swarm, event);
            }

            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {}", address);
            }
            // other event
            event => println!("event: {:?}", event),
        }
    }
}
