use smol::net::UdpSocket;

async fn run_server() -> smol::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0").await?;

    println!("Server listening on: {}", socket.local_addr()?);
    let mut buf = [0; 2048];
    loop {
        let (len, src) = socket.recv_from(&mut buf).await?;
        socket.send_to(&buf[..len], &src).await?;
    }
}

async fn run_client() -> smol::io::Result<()> {
    // get server address from command line
    let addr = std::env::args().nth(2).ok_or(smol::io::Error::new(
        smol::io::ErrorKind::InvalidInput,
        "invalid server address",
    ))?;
    let socket = UdpSocket::bind((smol::net::Ipv4Addr::UNSPECIFIED, 0)).await?;
    socket.connect(addr).await?;

    // send request
    let request = b"hello world";
    socket.send(request).await?;
    println!(
        "Sent request to server: {}",
        String::from_utf8_lossy(request)
    );

    // read reply
    let mut reply = vec![0; request.len()];
    socket.recv(&mut reply).await?;
    println!(
        "Read reply from server: {}",
        String::from_utf8_lossy(&reply)
    );
    Ok(())
}

fn main() -> smol::io::Result<()> {
    smol::block_on(async {
        // handle command line arguments
        if let Some(cmd) = std::env::args().nth(1) {
            match cmd.as_str() {
                "server" => return run_server().await,
                "client" => return run_client().await,
                _ => (),
            }
        }
        Ok(())
    })
}
