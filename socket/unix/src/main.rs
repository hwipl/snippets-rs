use std::io;
use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;

fn handle_client(stream: UnixStream) {
    let (mut reader, mut writer) = (&stream, &stream);
    if let Err(e) = io::copy(&mut reader, &mut writer) {
        println!("Error reading from client: {}", e);
    }
}

fn run_server() -> std::io::Result<()> {
    let listener = UnixListener::bind("sockfile.sock")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // spawn new thread that handles client stream
                thread::spawn(|| handle_client(stream));
            }
            Err(err) => {
                println!("stream error: {}", err);
                break;
            }
        }
    }
    Ok(())
}

fn run_client() -> std::io::Result<()> {
    let mut stream = UnixStream::connect("sockfile.sock")?;

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
    if let Some(cmd) = std::env::args().nth(1) {
        match cmd.as_str() {
            "server" => return run_server(),
            "client" => return run_client(),
            _ => (),
        }
    }
    Ok(())
}
