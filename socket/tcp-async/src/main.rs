use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn handle_client(mut stream: TcpStream) {
    let (mut reader, mut writer) = stream.split();
    if let Err(e) = tokio::io::copy(&mut reader, &mut writer).await {
        println!("Error reading from client: {}", e);
    }
}

async fn run_server() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;

    println!("Server listening on: {}", listener.local_addr()?);

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_client(stream).await;
        });
    }
}

async fn run_client() -> std::io::Result<()> {
    // get server address from command line
    let addr = std::env::args().nth(2).ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "invalid server address",
    ))?;
    let mut stream = TcpStream::connect(addr).await?;

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
