use zbus::{dbus_interface, Connection, Result};

// define ping interface
struct Ping;

#[dbus_interface(name = "org.ping.Ping")]
impl Ping {
    async fn ping(&self) -> String {
        println!("PING");
        format!("PONG")
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    // connect to session bus
    let connection = Connection::session().await?;

    // setup server
    connection
        .object_server_mut()
        .await
        .at("/org/ping/Ping", Ping)?;

    // request name
    connection.request_name("org.ping.Ping").await?;

    loop {
        // handle requests in the background
        std::thread::park();
    }
}
