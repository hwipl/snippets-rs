use futures_util::TryStreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full, StreamBody};
use hyper::body::{Bytes, Frame};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use hyper_util::server;
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::net::TcpListener;
use tokio_util::io::ReaderStream;

fn bad_request() -> Result<Response<BoxBody<Bytes, std::io::Error>>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Empty::new().map_err(|e| match e {}).boxed())
        .unwrap())
}

fn get_local_path(req: &Request<hyper::body::Incoming>) -> PathBuf {
    let mut path = req.uri().path();
    if path.len() > 0 {
        path = &path[1..];
    }
    env::current_dir().unwrap().join(path)
}

fn get_uri_path_parent(req: &Request<hyper::body::Incoming>) -> &str {
    let path = req.uri().path();
    match path.rsplit_once("/") {
        Some(("", _right)) => "/",
        Some((left, _right)) => left,
        None => path,
    }
}

async fn is_local_dir(req: &Request<hyper::body::Incoming>) -> bool {
    let path = get_local_path(&req);
    match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

async fn get_local_dir_html(req: &Request<hyper::body::Incoming>) -> String {
    let req_path = req.uri().path();
    let mut html = format!(
        "<!DOCTYPE html><html><head><title>{0}</title></head><body><ul><li><a href={1}>..</a></li>",
        req_path,
        get_uri_path_parent(&req),
    );
    let local_path = get_local_path(&req);
    if let Ok(mut entries) = tokio::fs::read_dir(local_path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(filetype) = entry.file_type().await {
                if filetype.is_symlink() {
                    continue;
                }
                let is_dir = match filetype.is_dir() {
                    true => "/",
                    false => "",
                };
                if let Some(name) = entry.file_name().to_str() {
                    let li = match req_path {
                        "/" => format!("<li><a href=/{0}>{0}{1}</a></li>", name, is_dir),
                        _ => format!(
                            "<li><a href={0}/{1}>{1}{2}</a></li>",
                            req_path, name, is_dir
                        ),
                    };
                    html += &li;
                }
            }
        }
    }
    html += "</ul></body></html>";
    html
}

async fn handle_get_dir(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, Infallible> {
    let html = get_local_dir_html(&req).await;
    let body = Full::from(html);
    Ok(Response::new(body.map_err(|e| match e {}).boxed()))
}

async fn handle_get_file(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, Infallible> {
    let path = get_local_path(&req);
    match File::open(path).await {
        Ok(file) => {
            let stream = ReaderStream::new(file);
            let body = StreamBody::new(stream.map_ok(Frame::data));
            Ok(Response::new(body.boxed()))
        }
        _ => bad_request(),
    }
}

async fn handle_get(
    remote_addr: SocketAddr,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, Infallible> {
    println!("{} {} {}", remote_addr, req.method(), req.uri().path());

    if is_local_dir(&req).await {
        handle_get_dir(req).await
    } else {
        handle_get_file(req).await
    }
}

async fn handle(
    remote_addr: SocketAddr,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, Infallible> {
    match req.method() {
        &Method::GET => handle_get(remote_addr, req).await,
        _ => bad_request(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // create listener
    let listener = TcpListener::bind(addr).await?;

    // main loop
    loop {
        // get connection from listeneer
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);

        // handle connection
        tokio::task::spawn(async move {
            if let Err(err) = server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(
                    io,
                    service_fn(|req: Request<hyper::body::Incoming>| async move {
                        handle(remote_addr, req).await
                    }),
                )
                .await
            {
                eprintln!("server error: {}", err);
            }
        });
    }
}
