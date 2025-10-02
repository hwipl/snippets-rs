use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

async fn handle_client(mut stream: UnixStream) {
    let (mut reader, mut writer) = stream.split();
    if let Err(e) = tokio::io::copy(&mut reader, &mut writer).await {
        println!("Error reading from client: {}", e);
    }
}

async fn run_server() -> std::io::Result<()> {
    let listener = UnixListener::bind("sockfile.sock")?;

    println!("Server listening on: {:?}", listener.local_addr()?);

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_client(stream).await;
        });
    }
}

async fn run_client() -> std::io::Result<()> {
    // connect to server
    let mut stream = UnixStream::connect("sockfile.sock").await?;

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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // handle command line arguments
    if let Some(cmd) = std::env::args().nth(1) {
        match cmd.as_str() {
            "server" => return run_server().await,
            "client" => return run_client().await,
            _ => (),
        }
    }
    Ok(())
}
