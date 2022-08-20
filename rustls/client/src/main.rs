use std::convert::TryInto;
use std::error::Error;
use std::io::{stdout, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
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

    // load certificates
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs()? {
        roots.add(&rustls::Certificate(cert.0))?;
    }

    // create config
    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();

    // connect to server
    let mut conn = rustls::ClientConnection::new(Arc::new(config), addr.as_str().try_into()?)?;
    let mut sock = connect(format!("{}:{}", addr, port))?;
    let mut tls = rustls::Stream::new(&mut conn, &mut sock);

    // send http request
    tls.write_all(
        format!(
            "GET / HTTP/1.1\r\n\
            Host: {}\r\n\
            Connection: close\r\n\
            Accept-Encoding: identity\r\n\
            \r\n",
            addr,
        )
        .as_bytes(),
    )?;

    // check cipher suite
    let ciphersuite = tls
        .conn
        .negotiated_cipher_suite()
        .ok_or("tls handshake failed")?;
    writeln!(
        &mut std::io::stderr(),
        "Current ciphersuite: {:?}",
        ciphersuite.suite()
    )?;

    // get http response
    let mut plaintext = Vec::new();
    tls.read_to_end(&mut plaintext)?;
    stdout().write_all(&plaintext)?;

    // get peer certificate
    let certificates = tls
        .conn
        .peer_certificates()
        .ok_or("getting peer certificates failed")?;
    println!("{:?}", certificates[0]);

    // get digest
    let digest = ring::digest::digest(&ring::digest::SHA256, certificates[0].as_ref());
    println!("{:?}", digest);

    Ok(())
}
