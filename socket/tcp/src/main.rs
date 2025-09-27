use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_client(stream: TcpStream) {
    let (mut reader, mut writer) = (&stream, &stream);
    if let Err(e) = std::io::copy(&mut reader, &mut writer) {
        println!("Error reading from client: {}", e);
    }
}

fn run_server() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;

    println!("Server listening on: {}", listener.local_addr()?);

    for stream in listener.incoming() {
        // spawn new thread that handles client stream
        let stream = stream?;
        thread::spawn(|| handle_client(stream));
    }
    Ok(())
}

fn run_client() -> std::io::Result<()> {
    // get server address from command line
    let addr = std::env::args().nth(2).ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "invalid server address",
    ))?;
    let mut stream = TcpStream::connect(addr)?;

    // sent request
    let request = b"hello world";
    stream.write_all(request)?;
    println!(
        "Sent request to server: {}",
        String::from_utf8_lossy(request)
    );

    // read reply
    let mut reply = vec![0; request.len()];
    stream.read_exact(&mut reply)?;
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
