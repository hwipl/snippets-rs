use axum::extract::{ConnectInfo, Request};
use axum::handler::HandlerWithoutStateExt;
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::services::ServeDir;

fn get_local_path(path: &str) -> PathBuf {
    let path = match path.len() {
        0 => path,
        _ => &path[1..],
    };
    env::current_dir().unwrap().join(path)
}

fn get_uri_path_parent(path: &str) -> &str {
    match path.rsplit_once("/") {
        Some(("", _right)) => "",
        Some((left, _right)) => left,
        None => path,
    }
}

async fn get_local_dir_html(req_path: &str) -> String {
    let mut html = format!(
        "<!DOCTYPE html><html><head><title>{0}</title></head><body><ul><li><a href={1}/>..</a></li>",
        req_path,
        get_uri_path_parent(req_path),
    );
    let local_path = get_local_path(req_path);
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
                        "/" => format!("<li><a href=/{0}{1}>{0}{1}</a></li>", name, is_dir),
                        _ => format!(
                            "<li><a href={0}/{1}{2}>{1}{2}</a></li>",
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

async fn is_local_dir(path: &str) -> bool {
    let path = get_local_path(path);
    match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

async fn fallback(request: Request) -> (StatusCode, Html<String>) {
    let path = request.uri().path().trim_end_matches('/');
    match is_local_dir(path).await {
        true => (StatusCode::OK, get_local_dir_html(path).await.into()),
        false => (StatusCode::NOT_FOUND, String::from("Not found").into()),
    }
}

#[tokio::main]
async fn main() {
    // create listener
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());

    // create fallback service
    let fallback = fallback.into_service();

    // create ServeDir service that prints client info
    let dir = env::current_dir().unwrap();
    let serve_dir = |ConnectInfo(addr): ConnectInfo<SocketAddr>, request: Request| async move {
        println!("{} {} {}", addr, request.method(), request.uri());
        ServeDir::new(dir).fallback(fallback).oneshot(request).await
    };

    // create app and start server
    let app = Router::new().nest_service("/", get(serve_dir));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
