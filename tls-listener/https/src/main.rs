use futures_util::StreamExt;
use hyper::server::accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use rcgen::generate_simple_self_signed;
use std::convert::Infallible;
use std::env;
use std::future::ready;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tls_listener::TlsListener;
use tokio::fs::File;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::TlsAcceptor;
use tokio_util::codec::{BytesCodec, FramedRead};

fn bad_request() -> Result<Response<Body>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::empty())
        .unwrap())
}

fn get_local_path(req: &Request<Body>) -> PathBuf {
    let mut path = req.uri().path();
    if path.len() > 0 {
        path = &path[1..];
    }
    env::current_dir().unwrap().join(path)
}

fn get_uri_path_parent(req: &Request<Body>) -> &str {
    let path = req.uri().path();
    match path.rsplit_once("/") {
        Some(("", _right)) => "/",
        Some((left, _right)) => left,
        None => path,
    }
}

async fn is_local_dir(req: &Request<Body>) -> bool {
    let path = get_local_path(&req);
    match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

async fn get_local_dir_html(req: &Request<Body>) -> String {
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

async fn handle_get_dir(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let html = get_local_dir_html(&req).await;
    let body = Body::from(html);
    Ok(Response::new(body))
}

async fn handle_get_file(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let path = get_local_path(&req);
    match File::open(path).await {
        Ok(file) => {
            let stream = FramedRead::new(file, BytesCodec::new());
            let body = Body::wrap_stream(stream);
            Ok(Response::new(body))
        }
        _ => bad_request(),
    }
}

async fn handle_get(
    remote_addr: SocketAddr,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    println!("{} {} {}", remote_addr, req.method(), req.uri().path());

    if is_local_dir(&req).await {
        handle_get_dir(req).await
    } else {
        handle_get_file(req).await
    }
}

async fn handle(remote_addr: SocketAddr, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match req.method() {
        &Method::GET => handle_get(remote_addr, req).await,
        _ => bad_request(),
    }
}

fn tls_acceptor() -> TlsAcceptor {
    // generate certificate and private key
    let cert = generate_simple_self_signed(Vec::new()).unwrap();
    let key = PrivateKey(cert.serialize_private_key_der());
    let cert = Certificate(cert.serialize_der().unwrap());

    Arc::new(
        ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .unwrap(),
    )
    .into()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // create request handler that uses remote address
    let make_service = make_service_fn(|conn: &tokio_rustls::server::TlsStream<AddrStream>| {
        let remote_addr = conn.get_ref().0.remote_addr();
        let service = service_fn(move |req| handle(remote_addr, req));

        async move { Ok::<_, Infallible>(service) }
    });

    // create tls listener for the http server
    let incoming = TlsListener::new(tls_acceptor(), AddrIncoming::bind(&addr)?).filter(|conn| {
        if let Err(err) = conn {
            eprintln!("Error: {:?}", err);
            ready(false)
        } else {
            ready(true)
        }
    });

    let server = Server::builder(accept::from_stream(incoming)).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
    Ok(())
}
