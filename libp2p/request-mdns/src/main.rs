// simple request response program with mdns discovery

use async_trait::async_trait;
use futures::executor::block_on;
use futures::prelude::*;
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed};
use libp2p::core::ProtocolName;
use libp2p::mdns::{Mdns, MdnsConfig, MdnsEvent};
use libp2p::request_response::{
    ProtocolSupport, RequestResponse, RequestResponseCodec, RequestResponseConfig,
    RequestResponseEvent, RequestResponseMessage,
};
use libp2p::swarm::{NetworkBehaviourEventProcess, Swarm};
use libp2p::{identity, NetworkBehaviour, PeerId};
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

/// Custom network behaviour with request response and mdns
#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
struct HelloBehaviour {
    request: RequestResponse<HelloCodec>,
    mdns: Mdns,
}

impl NetworkBehaviourEventProcess<RequestResponseEvent<HelloRequest, HelloResponse>>
    for HelloBehaviour
{
    // Called when `request` produces an event.
    fn inject_event(&mut self, message: RequestResponseEvent<HelloRequest, HelloResponse>) {
        // create messages
        let request = HelloRequest("hey".to_string().into_bytes());
        let response = HelloResponse("hi".to_string().into_bytes());

        // handle incoming messages
        if let RequestResponseEvent::Message { peer, message } = message {
            match message {
                // handle incoming request message, send back response
                RequestResponseMessage::Request { channel, .. } => {
                    println!("received request {:?} from {:?}", request, peer);
                    self.request
                        .send_response(channel, response.clone())
                        .unwrap();
                    return;
                }

                // handle incoming response message
                RequestResponseMessage::Response { response, .. } => {
                    println!("received response {:?} from {:?}", response, peer);
                    return;
                }
            }
        }

        // handle response sent event
        if let RequestResponseEvent::ResponseSent { peer, .. } = message {
            println!("sent response {:?} to {:?}", response, peer);
            return;
        }

        println!("request response error: {:?}", message);
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for HelloBehaviour {
    // Called when `mdns` produces an event.
    fn inject_event(&mut self, event: MdnsEvent) {
        let request = HelloRequest("hey".to_string().into_bytes());
        match event {
            MdnsEvent::Discovered(list) => {
                let mut new_peers: Vec<PeerId> = Vec::new();
                for (peer, addr) in list {
                    self.request.add_address(&peer, addr.clone());
                    if new_peers.contains(&peer) {
                        continue;
                    }
                    new_peers.push(peer);
                }

                for peer in new_peers {
                    self.request.send_request(&peer, request.clone());
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, addr) in list {
                    if !self.mdns.has_node(&peer) {
                        self.request.remove_address(&peer, &addr);
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

    // create mdns
    let mdns = block_on(Mdns::new(MdnsConfig::default()))?;

    // create request response
    let protocols = iter::once((HelloProtocol(), ProtocolSupport::Full));
    let cfg = RequestResponseConfig::default();
    let request = RequestResponse::new(HelloCodec(), protocols.clone(), cfg.clone());

    // create network behaviour
    let behaviour = HelloBehaviour { request, mdns };

    // create swarm
    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

    // listen on loopback interface and random port.
    swarm.listen_on("/ip6/::1/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

    // start main loop
    let mut listening = false;
    block_on(future::poll_fn(move |cx| loop {
        match swarm.poll_next_unpin(cx) {
            Poll::Ready(Some(event)) => println!("event: {:?}", event),
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
