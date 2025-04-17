use zbus::{interface, Connection, Result};

const PATH: &str = "/org/ping/Ping";
const IFACE: &str = "org.ping.Ping";

// define ping interface
struct Ping;

#[interface(name = "org.ping.Ping")]
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
    connection.object_server().at(PATH, Ping).await?;

    // request name
    connection.request_name(IFACE).await?;

    loop {
        // handle requests in the background
        std::thread::park();
    }
}
