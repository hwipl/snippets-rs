// simple request response program based on libp2p request and response ping test

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response::{Behaviour, Codec, Config, Event, Message, ProtocolSupport};
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId, SwarmBuilder};
use std::error::Error;
use std::time::Duration;
use std::{io, iter};

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
impl Codec for HelloCodec {
    type Protocol = HelloProtocol;
    type Request = HelloRequest;
    type Response = HelloResponse;

    async fn read_request<T>(&mut self, _: &HelloProtocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();
        io.take(1024).read_to_end(&mut vec).await?;
        if vec.is_empty() {
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
        // TODO: add length
        let mut vec = Vec::new();
        io.take(1024).read_to_end(&mut vec).await?;
        if vec.is_empty() {
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
        .with_behaviour(|_key| {
            // create request response network behaviour
            let protocols = iter::once((HelloProtocol(), ProtocolSupport::Full));
            let cfg = Config::default();
            Behaviour::<HelloCodec>::new(protocols.clone(), cfg.clone())
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(5)))
        .build();
    println!("Local peer id: {:?}", swarm.local_peer_id());

    // listen on loopback interface and random port.
    swarm.listen_on("/ip6/::1/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

    // create messages
    let request = HelloRequest("hey".to_string().into_bytes());
    let response = HelloResponse("hi".to_string().into_bytes());

    // connect to peer in first command line argument if present
    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote.clone())?;
        println!("Connecting to {}", addr);

        // send to peer id in second command line argument
        let peer_id = std::env::args().nth(2).ok_or("peer id missing")?;
        let peer_id: PeerId = peer_id.parse()?;
        swarm.add_peer_address(peer_id, remote.clone());
        swarm
            .behaviour_mut()
            .send_request(&peer_id, request.clone());
        println!("sent request {:?} to {:?}", request, peer_id);
    }

    // start main loop
    loop {
        match swarm.select_next_some().await {
            // handle incoming request message, send back response
            SwarmEvent::Behaviour(Event::Message {
                peer,
                connection_id: _,
                message:
                    Message::Request {
                        request, channel, ..
                    },
            }) => {
                println!("received request {:?} from {:?}", request, peer);
                swarm
                    .behaviour_mut()
                    .send_response(channel, response.clone())
                    .unwrap();
            }

            // handle incoming response message, stop
            SwarmEvent::Behaviour(Event::Message {
                peer,
                connection_id: _,
                message: Message::Response { response, .. },
            }) => {
                println!("received response {:?} from {:?}", response, peer);
                return Ok(());
            }

            // handle response sent event
            SwarmEvent::Behaviour(Event::ResponseSent { peer, .. }) => {
                println!("sent response {:?} to {:?}", response, peer);
            }

            // handle errors
            SwarmEvent::Behaviour(e) => {
                println!("error: {:?}", e);
            }

            // handle new listen address
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {}", address);
            }

            // ignore other events
            _ => (),
        }
    }
}
