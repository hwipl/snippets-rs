use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::net::{TcpListener, TcpStream};
use smol::stream::StreamExt;

async fn handle_client(stream: TcpStream) {
    let (reader, writer) = smol::io::split(stream);
    if let Err(e) = smol::io::copy(reader, writer).await {
        println!("Error reading from client: {}", e);
    }
}

async fn run_server() -> smol::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;

    println!("Server listening on: {}", listener.local_addr()?);

    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        smol::spawn(async move {
            handle_client(stream).await;
        })
        .detach();
    }
    Ok(())
}

async fn run_client() -> smol::io::Result<()> {
    // get server address from command line
    let addr = std::env::args().nth(2).ok_or(smol::io::Error::new(
        smol::io::ErrorKind::InvalidInput,
        "invalid server address",
    ))?;
    let mut stream = TcpStream::connect(addr).await?;

    // send request
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
