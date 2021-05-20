use async_std::os::unix::net::{UnixListener, UnixStream};
use async_std::prelude::*;
use async_std::task;

async fn handle_client(stream: UnixStream) {
    let mut stream = stream;
    let mut buffer = [0u8; 2048];
    loop {
        match stream.read(&mut buffer).await {
            Ok(num) => {
                if num == 0 {
                    return;
                }
                println!("Read {} bytes from client", num);
                match stream.write_all(&buffer[..num]).await {
                    Ok(()) => println!("Sent {} bytes to client", num),
                    Err(e) => {
                        println!("Error sending to client: {}", e);
                        return;
                    }
                }
            }
            Err(e) => {
                println!("Error reading from client: {}", e);
                return;
            }
        }
    }
}

async fn run_server() -> async_std::io::Result<()> {
    let listener = UnixListener::bind("sockfile.sock").await?;
    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        task::spawn(handle_client(stream?));
    }

    Ok(())
}

async fn run_client() -> async_std::io::Result<()> {
    let mut stream = UnixStream::connect("sockfile.sock").await?;

    // sent request
    let request = b"hello world";
    stream.write_all(request).await?;
    println!(
        "Sent request to server: {}",
        String::from_utf8_lossy(request)
    );

    // read reply
    let mut reply = vec![0; request.len()];
    stream.read_exact(&mut reply).await?;
    println!(
        "Read reply from server: {}",
        String::from_utf8_lossy(&reply)
    );

    Ok(())
}

fn main() -> async_std::io::Result<()> {
    if let Some(cmd) = std::env::args().nth(1) {
        match cmd.as_str() {
            "server" => return task::block_on(run_server()),
            "client" => return task::block_on(run_client()),
            _ => (),
        }
    }
    Ok(())
}
