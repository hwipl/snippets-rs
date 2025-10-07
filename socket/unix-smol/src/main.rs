use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::net::unix::{UnixListener, UnixStream};
use smol::stream::StreamExt;

async fn handle_client(stream: UnixStream) {
    let (mut reader, mut writer) = smol::io::split(stream);
    if let Err(e) = smol::io::copy(&mut reader, &mut writer).await {
        println!("Error reading from client: {}", e);
    }
}

async fn run_server() -> smol::io::Result<()> {
    let listener = UnixListener::bind("sockfile.sock")?;

    println!("Server listening on: {:?}", listener.local_addr()?);

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
