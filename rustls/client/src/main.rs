use std::convert::TryInto;
use std::error::Error;
use std::io::{stdout, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn Error>> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs()? {
        roots.add(&rustls::Certificate(cert.0))?;
    }

    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let mut conn = rustls::ClientConnection::new(Arc::new(config), "google.com".try_into()?)?;
    let mut sock = TcpStream::connect("google.com:443")?;
    let mut tls = rustls::Stream::new(&mut conn, &mut sock);
    tls.write_all(
        concat!(
            "GET / HTTP/1.1\r\n",
            "Host: google.com\r\n",
            "Connection: close\r\n",
            "Accept-Encoding: identity\r\n",
            "\r\n"
        )
        .as_bytes(),
    )?;
    let ciphersuite = tls
        .conn
        .negotiated_cipher_suite()
        .ok_or("tls handshake failed")?;
    writeln!(
        &mut std::io::stderr(),
        "Current ciphersuite: {:?}",
        ciphersuite.suite()
    )?;
    let mut plaintext = Vec::new();
    tls.read_to_end(&mut plaintext)?;
    stdout().write_all(&plaintext)?;

    Ok(())
}
