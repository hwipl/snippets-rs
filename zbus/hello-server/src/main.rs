use async_std::task;
use std::time::Duration;
use zbus::{interface, object_server::SignalEmitter, Connection, Result};

const NAME: &str = IFACE;
const PATH: &str = "/org/world/Hello";
const IFACE: &str = "org.world.Hello";

// set hello interface
struct Hello {}

#[interface(name = "org.world.Hello")]
impl Hello {
    #[zbus(signal)]
    async fn hello(emitter: &SignalEmitter<'_>, ping: &str) -> Result<()>;

    async fn hi(
        &self,
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
    conn.object_server().at(PATH, interface).await?;

    // periodically send hello world signal,
    // handle method calls in the background
    loop {
        task::sleep(Duration::from_secs(5)).await;
        Hello::hello(&SignalEmitter::new(&conn, PATH)?, "Hello, world!").await?;
    }
}
