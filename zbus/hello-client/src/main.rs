use futures_util::stream::StreamExt;
use std::env;
use zbus::{dbus_proxy, Connection, Result};

#[dbus_proxy(
    default_service = "org.world.Hello",
    interface = "org.world.Hello",
    default_path = "/org/world/Hello"
)]
trait Hello {
    #[dbus_proxy(signal)]
    fn hello(&self, msg: &str) -> Result<()>;

    fn hi(&self, name: &str) -> Result<String>;
}

#[async_std::main]
async fn main() -> Result<()> {
    // get own name from command line
    let mut name = String::from("Nobody");
    if let Some(arg) = env::args().nth(1) {
        name = arg;
    }

    // connect to session bus
    let conn = Connection::session().await?;

    // create proxy and hello signal stream
    let proxy = HelloProxy::new(&conn).await?;
    let mut hello_stream = proxy.receive_hello().await?;

    // handle incoming hello signals
    while let Some(signal) = hello_stream.next().await {
        // print msg argument of signal
        let args = signal.args()?;
        println!("{}", args.msg());

        // call hi method with own name and print returned greeting
        println!("{}", proxy.hi(&name).await?);
    }

    Ok(())
}
