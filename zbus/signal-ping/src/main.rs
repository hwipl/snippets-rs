use async_std::task;
use futures_util::stream::StreamExt;
use std::time::Duration;
use zbus::{dbus_proxy, Connection, Result};

const PATH: &str = "/org/ping/Ping";
const IFACE: &str = "org.ping.Ping";

#[dbus_proxy(
    default_service = "org.ping.Ping",
    interface = "org.ping.Ping",
    default_path = "/org/ping/Ping"
)]
trait Ping {
    #[dbus_proxy(signal)]
    fn ping(&self, ping: &str) -> Result<()>;
}

/// periodically send ping signal
async fn send_ping(conn: Connection) -> Result<()> {
    loop {
        task::sleep(Duration::from_secs(1)).await;
        conn.emit_signal(None::<()>, PATH, IFACE, "Ping", &"PING")
            .await?;
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    // connect to session bus and request well known name
    let conn = Connection::session().await?;
    conn.request_name("org.ping.Ping").await?;

    // create proxy and ping signal stream
    let proxy = PingProxy::new(&conn).await?;
    let mut ping_stream = proxy.receive_ping().await?;

    // spawn task that sends ping signals
    task::spawn(send_ping(conn));

    // handle incoming ping signals
    while let Some(signal) = ping_stream.next().await {
        let args = signal.args()?;
        println!("{}", args.ping());
    }

    Ok(())
}
