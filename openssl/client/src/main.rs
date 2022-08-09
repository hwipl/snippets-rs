use openssl::ssl::{SslConnector, SslMethod};
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn Error>> {
    // get address from command line
    let addr = match std::env::args().nth(1) {
        Some(addr) => addr,
        None => "www.rust-lang.org".to_string(),
    };

    // get port from command line
    let port = match std::env::args().nth(2) {
        Some(port) => port,
        None => "443".to_string(),
    };

    // connect to server
    let connector = SslConnector::builder(SslMethod::tls())?.build();
    let stream = TcpStream::connect(format!("{}:{}", addr, port))?;
    let mut stream = connector.connect(addr.as_str(), stream)?;

    // run http request
    stream.write_all(
        format!(
            "GET / HTTP/1.1\r\n\
            Host: {}\r\n\
            Connection: close\r\n\
            Accept-Encoding: identity\r\n\
            \r\n",
            addr
        )
        .as_bytes(),
    )?;
    let mut res = vec![];
    stream.read_to_end(&mut res)?;
    println!("{}", String::from_utf8_lossy(&res));

    // get peer certificate
    let certificate = stream.ssl().peer_certificate();
    println!("{:?}", certificate);

    Ok(())
}
