use std::convert::TryInto;
use std::error::Error;
use std::io::{stdout, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn Error>> {
    // get address from command line
    let addr = match std::env::args().nth(1) {
        Some(addr) => addr,
        None => "www.rust-lang.org".to_string(),
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
    let mut sock = TcpStream::connect(format!("{}:443", addr))?;
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

    Ok(())
}
