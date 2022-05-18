// simple request response program based on libp2p request and response ping test

use async_trait::async_trait;
use futures::executor::block_on;
use futures::prelude::*;
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed};
use libp2p::core::ProtocolName;
use libp2p::request_response::{
    ProtocolSupport, RequestResponse, RequestResponseCodec, RequestResponseConfig,
    RequestResponseEvent, RequestResponseMessage,
};
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId};
use std::error::Error;
use std::task::Poll;
use std::{io, iter};

/// Hello protocol for the request response behaviour
#[derive(Debug, Clone)]
struct HelloProtocol();

impl ProtocolName for HelloProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/hello/0.0.1".as_bytes()
    }
}

/// Hello codec for the request response behaviour
#[derive(Clone)]
struct HelloCodec();

#[async_trait]
impl RequestResponseCodec for HelloCodec {
    type Protocol = HelloProtocol;
    type Request = HelloRequest;
    type Response = HelloResponse;

    async fn read_request<T>(&mut self, _: &HelloProtocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_length_prefixed(io, 1024)
            .map(|res| match res {
                Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                Ok(vec) if vec.is_empty() => Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(vec) => Ok(HelloRequest(vec)),
            })
            .await
    }

    async fn read_response<T>(
        &mut self,
        _: &HelloProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_length_prefixed(io, 1024)
            .map(|res| match res {
                Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                Ok(vec) if vec.is_empty() => Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(vec) => Ok(HelloResponse(vec)),
            })
            .await
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
        write_length_prefixed(io, data).await
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
        write_length_prefixed(io, data).await
    }
}

/// Request message
#[derive(Debug, Clone, PartialEq, Eq)]
struct HelloRequest(Vec<u8>);

/// Response message
#[derive(Debug, Clone, PartialEq, Eq)]
struct HelloResponse(Vec<u8>);

fn main() -> Result<(), Box<dyn Error>> {
    // create key and peer id
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    // create transport
    let transport = block_on(libp2p::development_transport(local_key))?;

    // create request response network behaviour
    let protocols = iter::once((HelloProtocol(), ProtocolSupport::Full));
    let cfg = RequestResponseConfig::default();
    let behaviour = RequestResponse::new(HelloCodec(), protocols.clone(), cfg.clone());

    // create swarm
    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

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
        swarm.behaviour_mut().add_address(&peer_id, remote.clone());
        swarm
            .behaviour_mut()
            .send_request(&peer_id, request.clone());
        println!("sent request {:?} to {:?}", request, peer_id);
    }

    // start main loop
    let mut listening = false;
    block_on(future::poll_fn(move |cx| loop {
        match swarm.poll_next_unpin(cx) {
            Poll::Ready(Some(event)) => match event {
                // handle incoming request message, send back response
                SwarmEvent::Behaviour(RequestResponseEvent::Message {
                    peer,
                    message:
                        RequestResponseMessage::Request {
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
                SwarmEvent::Behaviour(RequestResponseEvent::Message {
                    peer,
                    message: RequestResponseMessage::Response { response, .. },
                }) => {
                    println!("received response {:?} from {:?}", response, peer);
                    return Poll::Ready(());
                }

                // handle response sent event
                SwarmEvent::Behaviour(RequestResponseEvent::ResponseSent { peer, .. }) => {
                    println!("sent response {:?} to {:?}", response, peer);
                }

                // handle errors
                SwarmEvent::Behaviour(e) => {
                    println!("error: {:?}", e);
                }

                // ignore other events
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
