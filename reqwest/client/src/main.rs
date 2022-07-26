use reqwest::Result;

#[async_std::main]
async fn main() -> Result<()> {
    // get url from command line
    let addr = match std::env::args().nth(1) {
        Some(addr) => addr,
        None => "https://www.rust-lang.org".to_string(),
    };

    // run get request
    let body = reqwest::get(addr).await?.text().await?;

    // print returned body
    println!("body = {:?}", body);

    Ok(())
}
