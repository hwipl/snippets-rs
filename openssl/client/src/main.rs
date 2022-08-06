use openssl::ssl::{SslConnector, SslMethod};
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn Error>> {
    let connector = SslConnector::builder(SslMethod::tls())?.build();

    let stream = TcpStream::connect("google.com:443")?;
    let mut stream = connector.connect("google.com", stream)?;

    stream.write_all(b"GET / HTTP/1.0\r\n\r\n")?;
    let mut res = vec![];
    stream.read_to_end(&mut res)?;
    println!("{}", String::from_utf8_lossy(&res));

    Ok(())
}
