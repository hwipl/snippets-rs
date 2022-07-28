use reqwest::Result;
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<()> {
    // get url from command line
    let addr = match std::env::args().nth(1) {
        Some(addr) => addr,
        None => "https://www.rust-lang.org".to_string(),
    };

    // create client
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(5))
        .build()?;

    // run get request
    let body = client.get(addr).send().await?.text().await?;

    // print returned body
    println!("body = {:?}", body);

    Ok(())
}
