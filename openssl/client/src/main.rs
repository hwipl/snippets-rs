use openssl::hash::MessageDigest;
use openssl::ssl::{SslConnector, SslMethod};
use std::error::Error;
use std::fmt::Write as FmtWrite;
use std::io::{Read, Write as IoWrite};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

fn connect(addr: String) -> Result<TcpStream, Box<dyn Error>> {
    // try to resolve address to socket addresses
    let sock_addrs = addr.to_socket_addrs()?;
    if sock_addrs.len() == 0 {
        return Err("could not resolve address".into());
    }

    // try connecting to each socket address with a short connect timeout,
    // set read and write timeout on first successfull connection and return it
    for sock_addr in sock_addrs {
        if let Ok(sock) = TcpStream::connect_timeout(&sock_addr, CONNECT_TIMEOUT) {
            sock.set_read_timeout(Some(READ_TIMEOUT))?;
            sock.set_write_timeout(Some(WRITE_TIMEOUT))?;
            return Ok(sock);
        };
    }

    Err("failed to connect".into())
}

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
    let stream = connect(format!("{}:{}", addr, port))?;
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
    let certificate = stream
        .ssl()
        .peer_certificate()
        .ok_or("could not get certificate")?;
    println!("{:?}", certificate);

    // get digest
    let digest = certificate.digest(MessageDigest::sha256())?;
    let mut digest_hex = String::new();
    for byte in digest.iter() {
        write!(&mut digest_hex, "{:X}", byte)?;
    }
    println!("Digest: {}", digest_hex);

    Ok(())
}
