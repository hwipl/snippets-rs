use reqwest::Result;
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<()> {
    // create client
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(5))
        .build()?;

    // get url from command line
    let addr = match std::env::args().nth(2) {
        Some(addr) => addr,
        None => "https://www.rust-lang.org".to_string(),
    };

    // create request
    let request = match std::env::args().nth(1) {
        Some(cmd) => match cmd.as_str() {
            "delete" => client.delete(addr),
            "get" => client.get(addr),
            "head" => client.head(addr),
            "patch" => client.patch(addr),
            "post" => client.post(addr),
            "put" => client.put(addr),
            _ => panic!("invalid request"),
        },
        None => client.get(addr),
    };

    // wait for response
    let response = request.send().await?;

    // print status and headers
    println!("{}", response.status());
    for (header, value) in response.headers() {
        println!("{}: {:?}", header, value);
    }
    println!();

    // print returned body
    let body = response.text().await?;
    if body != "" {
        println!("{}", body);
    }

    Ok(())
}
