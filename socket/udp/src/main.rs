use std::net::{Ipv4Addr, UdpSocket};

fn run_server() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;

    println!("Server listening on: {}", socket.local_addr()?);
    let mut buf = [0; 2048];
    loop {
        let (len, src) = socket.recv_from(&mut buf)?;
        socket.send_to(&buf[..len], &src)?;
    }
}

fn run_client() -> std::io::Result<()> {
    // get server address from command line
    let addr = std::env::args().nth(2).ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "invalid server address",
    ))?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.connect(addr)?;

    // sent request
    let request = b"hello world";
    socket.send(request)?;
    println!(
        "Sent request to server: {}",
        String::from_utf8_lossy(request)
    );

    // read reply
    let mut reply = vec![0; request.len()];
    socket.recv(&mut reply)?;
    println!(
        "Read reply from server: {}",
        String::from_utf8_lossy(&reply)
    );
    Ok(())
}

fn main() -> std::io::Result<()> {
    // handle command line arguments
    if let Some(cmd) = std::env::args().nth(1) {
        match cmd.as_str() {
            "server" => return run_server(),
            "client" => return run_client(),
            _ => (),
        }
    }
    Ok(())
}
