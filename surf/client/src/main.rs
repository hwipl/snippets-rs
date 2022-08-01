use std::time::Duration;
use surf::{Client, Config, Result};

#[async_std::main]
async fn main() -> Result<()> {
    // create client
    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()?;

    // get url from command line
    let addr = match std::env::args().nth(2) {
        Some(addr) => addr,
        None => "https://www.rust-lang.org".to_string(),
    };

    // create request
    let request = match std::env::args().nth(1) {
        Some(cmd) => match cmd.as_str() {
            "get" => client.get(addr),
            _ => panic!("invalid request"),
        },
        None => client.get(addr),
    };

    // wait for response
    let mut res = request.await?;

    // print status
    let status = res.status();
    println!("{} ({0:?})", status);

    // print version
    if let Some(version) = res.version() {
        println!("{:?}", version);
    }
    println!();

    // print headers
    for (header, value) in res.iter() {
        println!("{}: {}", header, value);
    }

    // print body
    println!("\n");
    let body = res.body_string().await?;
    println!("{}", body);

    Ok(())
}
