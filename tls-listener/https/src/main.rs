use futures_util::TryStreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full, StreamBody};
use hyper::body::{Bytes, Frame, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use hyper_util::server;
use rcgen::generate_simple_self_signed;
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tls_listener::TlsListener;
use tokio::fs::File;
use tokio::net::TcpListener;
use tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use tokio_util::io::ReaderStream;

fn bad_request() -> Result<Response<BoxBody<Bytes, std::io::Error>>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Empty::new().map_err(|e| match e {}).boxed())
        .unwrap())
}

/// remove extra slashes from request path.
fn remove_extra_slashes(path: &str) -> String {
    let mut out = String::new();
    let mut previous_slash = false;
    for c in path.chars() {
        if c == '/' {
            if previous_slash {
                // skip duplicate slashes
                continue;
            }

            previous_slash = true;
        } else {
            previous_slash = false;
        }
        out.push(c);
    }
    out
}

/// get request path without extra slashes and without slash at the end.
fn get_req_path(req: &Request<Incoming>) -> String {
    remove_extra_slashes(req.uri().path().trim_end_matches('/'))
}

fn get_local_path(req: &Request<Incoming>) -> PathBuf {
    let path = get_req_path(req);
    let mut path = path.as_str();
    if path.len() > 0 {
        path = &path[1..];
    }
    env::current_dir().unwrap().join(path)
}

fn get_uri_path_parent(req: &Request<Incoming>) -> String {
    let path = get_req_path(req);
    match path.rsplit_once("/") {
        Some(("", _right)) => "".into(),
        Some((left, _right)) => left.into(),
        None => "".into(),
    }
}

async fn is_local_dir(req: &Request<Incoming>) -> bool {
    let path = get_local_path(&req);
    match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

async fn get_local_dir_html(req: &Request<Incoming>) -> String {
    let req_path = get_req_path(req);
    let mut html = format!(
        "<!DOCTYPE html><html><head><title>{0}/</title></head><body><ul><li><a href={1}/>..</a></li>",
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
                    html += &format!(
                        "<li><a href={0}/{1}{2}>{1}{2}</a></li>",
                        req_path, name, is_dir
                    );
                }
            }
        }
    }
    html += "</ul></body></html>";
    html
}

async fn handle_get_dir(
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, Infallible> {
    let html = get_local_dir_html(&req).await;
    let body = Full::from(html);
    Ok(Response::new(body.map_err(|e| match e {}).boxed()))
}

async fn handle_get_file(
    req: Request<Incoming>,
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
    req: Request<Incoming>,
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
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, Infallible> {
    match req.method() {
        &Method::GET => handle_get(remote_addr, req).await,
        _ => bad_request(),
    }
}

fn tls_acceptor() -> TlsAcceptor {
    // generate certificate and private key
    let cert = generate_simple_self_signed(Vec::new()).unwrap();
    let key = PrivatePkcs8KeyDer::from(cert.serialize_private_key_der());
    let cert = cert.serialize_der().unwrap();

    Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert.into()], key.into())
            .unwrap(),
    )
    .into()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // create listener
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let mut listener = TlsListener::new(tls_acceptor(), TcpListener::bind(addr).await?);

    println!(
        "Serving HTTP on {} port {} (https://{}/)...",
        addr.ip(),
        addr.port(),
        addr
    );

    // main loop
    loop {
        // get connection from listener
        let (stream, remote_addr) = match listener.accept().await {
            Ok((stream, remote_addr)) => (stream, remote_addr),
            Err(err) => {
                eprintln!("Error: {:?}", err);
                continue;
            }
        };
        let io = TokioIo::new(stream);

        // set service function
        let service = move |req: hyper::Request<hyper::body::Incoming>| async move {
            handle(remote_addr, req).await
        };

        // handle connection
        tokio::task::spawn(async move {
            if let Err(err) = server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(io, service_fn(service))
                .await
            {
                eprintln!("server error: {}", err);
            }
        });
    }
}
