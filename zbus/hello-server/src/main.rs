use async_std::task;
use std::time::Duration;
use zbus::{dbus_interface, Connection, MessageHeader, ObjectServer, Result, SignalContext};

const NAME: &str = IFACE;
const PATH: &str = "/org/world/Hello";
const IFACE: &str = "org.world.Hello";

// set hello interface
struct Hello {}

#[dbus_interface(name = "org.world.Hello")]
impl Hello {
    #[dbus_interface(signal)]
    async fn hello(signal_ctxt: &SignalContext<'_>, ping: &str) -> Result<()>;

    async fn hi(
        &self,
        #[zbus(header)] _hdr: MessageHeader<'_>,
        #[zbus(signal_context)] _ctxt: SignalContext<'_>,
        #[zbus(object_server)] _server: &ObjectServer,
        name: &str,
    ) -> zbus::fdo::Result<String> {
        // send back greeting
        Ok(format!("Hi, {}!", name))
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    // connect to session bus and request well-known name
    let conn = Connection::session().await?;
    conn.request_name(NAME).await?;

    // register interface
    let interface = Hello {};
    conn.object_server_mut().await.at(PATH, interface)?;

    // periodically send hello world signal,
    // handle method calls in the background
    loop {
        task::sleep(Duration::from_secs(5)).await;
        Hello::hello(&SignalContext::new(&conn, PATH)?, "Hello, world!").await?;
    }
}
