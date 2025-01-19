// hello world protocol based on the libp2p ping protocol

use futures::future::BoxFuture;
use futures::prelude::*;
use futures_timer::Delay;
use libp2p::core::{transport::PortUse, Endpoint, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use libp2p::swarm::handler::{
    ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
};
use libp2p::swarm::{
    ConnectionDenied, ConnectionHandler, ConnectionHandlerEvent, ConnectionId, FromSwarm,
    NetworkBehaviour, Stream, StreamUpgradeError, SubstreamProtocol, ToSwarm,
};
use libp2p::{Multiaddr, PeerId, StreamProtocol};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::task::{Context, Poll};
use std::time::Duration;
use std::{io, iter};
use void::Void;

/// `HelloWorld` network behaviour.
pub struct HelloWorld {
    // Queue of events to yield to the swarm.
    events: VecDeque<HelloWorldEvent>,
}

/// Event generated by the `HelloWorld` network behaviour.
#[derive(Debug)]
pub struct HelloWorldEvent {
    /// The peer ID of the remote.
    pub peer: PeerId,
    /// The result of an inbound or outbound hello world message.
    pub result: HelloWorldResult,
}

/// The result of an inbound or outbound hello world message.
pub type HelloWorldResult = Result<HelloWorldSuccess, HelloWorldFailure>;

/// The successful result of processing an inbound or outbound hello world message.
#[derive(Debug)]
pub enum HelloWorldSuccess {
    /// Received a hello world message.
    Received,
    /// Sent a hello world message.
    Sent,
}

/// An outbound hello world failure.
#[derive(Debug)]
pub enum HelloWorldFailure {
    /// The hello world timed out.
    Timeout,
    /// The hello world failed for reasons other than a timeout.
    Other {
        error: Box<dyn std::error::Error + Send + 'static>,
    },
}

impl fmt::Display for HelloWorldFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HelloWorldFailure::Timeout => f.write_str("Hello world timeout"),
            HelloWorldFailure::Other { error } => write!(f, "Hello world error: {}", error),
        }
    }
}

impl Error for HelloWorldFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            HelloWorldFailure::Timeout => None,
            HelloWorldFailure::Other { error } => Some(&**error),
        }
    }
}

impl HelloWorld {
    /// Creates a new `HelloWorld` network behaviour.
    pub fn new() -> Self {
        HelloWorld {
            events: VecDeque::new(),
        }
    }
}

impl NetworkBehaviour for HelloWorld {
    type ConnectionHandler = HelloWorldHandler;
    type ToSwarm = HelloWorldEvent;

    fn handle_established_inbound_connection(
        &mut self,
        _: ConnectionId,
        _: PeerId,
        _: &Multiaddr,
        _: &Multiaddr,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        Ok(HelloWorldHandler::new())
    }

    fn handle_established_outbound_connection(
        &mut self,
        _: ConnectionId,
        _: PeerId,
        _: &Multiaddr,
        _: Endpoint,
        _: PortUse,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        Ok(HelloWorldHandler::new())
    }

    fn on_connection_handler_event(
        &mut self,
        peer: PeerId,
        _: ConnectionId,
        result: HelloWorldResult,
    ) {
        self.events.push_front(HelloWorldEvent { peer, result })
    }

    fn on_swarm_event(&mut self, _event: FromSwarm) {}

    fn poll(&mut self, _: &mut Context<'_>) -> Poll<ToSwarm<HelloWorldEvent, Void>> {
        if let Some(e) = self.events.pop_back() {
            Poll::Ready(ToSwarm::GenerateEvent(e))
        } else {
            Poll::Pending
        }
    }
}

/// Protocol handler that handles sending hello world messages to the remote at a regular period.
///
/// If the remote doesn't respond, produces an error that closes the connection.
pub struct HelloWorldHandler {
    /// The timer used for the delay to the next hello world as well as
    /// the hello world timeout.
    timer: Delay,
    /// Outbound hello world failures that are pending to be processed by `poll()`.
    pending_errors: VecDeque<HelloWorldFailure>,
    /// The outbound hello world state.
    outbound: Option<HelloWorldState>,
    /// The inbound hello world handler, i.e. if there is an inbound
    /// substream, this is always a future that waits for the
    /// next inbound hello world message.
    inbound: Option<HelloWorldFuture>,
}

impl HelloWorldHandler {
    /// Builds a new `HelloWorldHandler`.
    pub fn new() -> Self {
        HelloWorldHandler {
            timer: Delay::new(Duration::new(5, 0)),
            pending_errors: VecDeque::with_capacity(2),
            outbound: None,
            inbound: None,
        }
    }
}

impl ConnectionHandler for HelloWorldHandler {
    type FromBehaviour = Void;
    type ToBehaviour = HelloWorldResult;
    type InboundProtocol = HelloWorldProtocol;
    type OutboundProtocol = HelloWorldProtocol;
    type OutboundOpenInfo = ();
    type InboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<HelloWorldProtocol, ()> {
        SubstreamProtocol::new(HelloWorldProtocol, ())
    }

    fn on_behaviour_event(&mut self, _event: Self::FromBehaviour) {}

    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<'_, Self::InboundProtocol, Self::OutboundProtocol>,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {
                protocol,
                info: _,
            }) => {
                self.inbound = Some(recv_hello_world(protocol).boxed());
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol,
                info: _,
            }) => {
                self.timer.reset(Duration::new(5, 0));
                self.outbound = Some(HelloWorldState::HelloWorld(
                    send_hello_world(protocol).boxed(),
                ));
            }
            ConnectionEvent::DialUpgradeError(DialUpgradeError { info: _, error }) => {
                self.outbound = None; // Request a new substream on the next `poll`.
                self.pending_errors.push_front(match error {
                    // Note: This timeout only covers protocol negotiation.
                    StreamUpgradeError::Timeout => HelloWorldFailure::Timeout,
                    e => HelloWorldFailure::Other { error: Box::new(e) },
                })
            }
            _ => {}
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ConnectionHandlerEvent<HelloWorldProtocol, (), HelloWorldResult>> {
        // Respond to inbound hello world messages.
        if let Some(fut) = self.inbound.as_mut() {
            match fut.poll_unpin(cx) {
                Poll::Pending => {}
                Poll::Ready(Err(e)) => {
                    log::debug!("Inbound hello world error: {:?}", e);
                    self.inbound = None;
                }
                Poll::Ready(Ok(stream)) => {
                    // hello world from a remote peer received, wait for next.
                    self.inbound = Some(recv_hello_world(stream).boxed());
                    return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(Ok(
                        HelloWorldSuccess::Received,
                    )));
                }
            }
        }

        loop {
            // Check for outbound hello world failures.
            if let Some(error) = self.pending_errors.pop_back() {
                log::debug!("Hello world failure: {:?}", error);
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(Err(error)));
            }

            // Continue outbound hello world messages.
            match self.outbound.take() {
                Some(HelloWorldState::HelloWorld(mut hello)) => match hello.poll_unpin(cx) {
                    Poll::Pending => {
                        if self.timer.poll_unpin(cx).is_ready() {
                            self.pending_errors.push_front(HelloWorldFailure::Timeout);
                        } else {
                            self.outbound = Some(HelloWorldState::HelloWorld(hello));
                            break;
                        }
                    }
                    Poll::Ready(Ok(stream)) => {
                        self.timer.reset(Duration::new(5, 0));
                        self.outbound = Some(HelloWorldState::Idle(stream));
                        return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(Ok(
                            HelloWorldSuccess::Sent,
                        )));
                    }
                    Poll::Ready(Err(e)) => {
                        self.pending_errors
                            .push_front(HelloWorldFailure::Other { error: Box::new(e) });
                    }
                },
                Some(HelloWorldState::Idle(stream)) => match self.timer.poll_unpin(cx) {
                    Poll::Pending => {
                        self.outbound = Some(HelloWorldState::Idle(stream));
                        break;
                    }
                    Poll::Ready(()) => {
                        self.timer.reset(Duration::new(5, 0));
                        self.outbound = Some(HelloWorldState::HelloWorld(
                            send_hello_world(stream).boxed(),
                        ));
                    }
                },
                Some(HelloWorldState::OpenStream) => {
                    self.outbound = Some(HelloWorldState::OpenStream);
                    break;
                }
                None => {
                    self.outbound = Some(HelloWorldState::OpenStream);
                    let protocol = SubstreamProtocol::new(HelloWorldProtocol, ())
                        .with_timeout(Duration::new(5, 0));
                    return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                        protocol,
                    });
                }
            }
        }

        Poll::Pending
    }
}

type HelloWorldFuture = BoxFuture<'static, Result<Stream, io::Error>>;

/// The current state w.r.t. outbound hello world messages.
enum HelloWorldState {
    /// A new substream is being negotiated for the hello world protocol.
    OpenStream,
    /// The substream is idle, waiting to send the next hello world.
    Idle(Stream),
    /// A hello world is being sent.
    HelloWorld(HelloWorldFuture),
}

/// `HelloWorld` protocol.
#[derive(Default, Debug, Copy, Clone)]
pub struct HelloWorldProtocol;

const HELLO_WORLD_MSG: &[u8] = b"hi";

impl UpgradeInfo for HelloWorldProtocol {
    type Info = StreamProtocol;
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(StreamProtocol::new("/hello/world/1.0.0"))
    }
}

impl InboundUpgrade<Stream> for HelloWorldProtocol {
    type Output = Stream;
    type Error = Void;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, stream: Stream, _: Self::Info) -> Self::Future {
        future::ok(stream)
    }
}

impl OutboundUpgrade<Stream> for HelloWorldProtocol {
    type Output = Stream;
    type Error = Void;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, stream: Stream, _: Self::Info) -> Self::Future {
        future::ok(stream)
    }
}

/// Sends a hello world message.
pub async fn send_hello_world<S>(mut stream: S) -> io::Result<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    stream.write_all(HELLO_WORLD_MSG).await?;
    stream.flush().await?;
    Ok(stream)
}

/// Waits for a hello world message.
pub async fn recv_hello_world<S>(mut stream: S) -> io::Result<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut payload = [0u8; HELLO_WORLD_MSG.len()];
    stream.read_exact(&mut payload).await?;
    Ok(stream)
}
