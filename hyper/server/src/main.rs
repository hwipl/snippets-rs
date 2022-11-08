use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs::File;
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
                if let Some(name) = entry.file_name().to_str() {
                    let href = match req_path {
                        "/" => format!("<li><a href=/{0}>{0}</a></li>", name),
                        _ => format!("<li><a href={0}/{1}>{1}</a></li>", req_path, name),
                    };
                    html += &href;
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

async fn handle_get(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    println!("{} {}", req.method(), req.uri().path());

    if is_local_dir(&req).await {
        handle_get_dir(req).await
    } else {
        handle_get_file(req).await
    }
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match req.method() {
        &Method::GET => handle_get(req).await,
        _ => bad_request(),
    }
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
    let server = Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
