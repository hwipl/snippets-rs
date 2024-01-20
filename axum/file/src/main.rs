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

/// convert the request path to a local path name.
fn get_local_path(path: &str) -> PathBuf {
    env::current_dir().unwrap().join(&path[1..])
}

/// get parent directory of request path.
fn get_uri_path_parent(path: &str) -> &str {
    match path[..path.len() - 1].rsplit_once("/") {
        Some(("", _right)) => "",
        Some((left, _right)) => left,
        None => "",
    }
}

/// get local directory listing as html for request path.
async fn get_local_dir_html(path: &str) -> String {
    // create header and start directory listing with the ".." entry
    let mut html = format!(
        "<!DOCTYPE html><html><head><title>{0}</title></head><body><ul><li><a href={1}/>..</a></li>",
        path,
        get_uri_path_parent(path)
    );

    // add content of directory to the directory listing
    let local_path = get_local_path(path);
    if let Ok(mut entries) = tokio::fs::read_dir(local_path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(filetype) = entry.file_type().await {
                // skip symlinks
                if filetype.is_symlink() {
                    continue;
                }

                // add trailing "/" to subdirectories
                let is_dir = match filetype.is_dir() {
                    true => "/",
                    false => "",
                };

                // add entry to directory listing
                if let Some(name) = entry.file_name().to_str() {
                    html += &format!("<li><a href={0}{1}{2}>{1}{2}</a></li>", path, name, is_dir);
                }
            }
        }
    }

    // close directory listing and other html tags
    html += "</ul></body></html>";
    html
}

/// return whether request path is a local directory.
async fn is_local_dir(path: &str) -> bool {
    let path = get_local_path(path);
    match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
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

/// fallback handler that returns a directory listing if client requested a local directory or
/// otherwise returns a "not found" error.
async fn fallback(request: Request) -> (StatusCode, Html<String>) {
    // get request path and remove extra slashes
    let path = request.uri().path();
    let path = remove_extra_slashes(path);

    match is_local_dir(&path).await {
        true => (StatusCode::OK, get_local_dir_html(&path).await.into()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_uri_path_parent() {
        for (path, want) in vec![
            // root dir
            ("/", ""),
            ("/1/", ""),
            // not root dir
            ("/1/2/", "/1"),
            ("/1/2/3/", "/1/2"),
            ("/1/2/3/4/", "/1/2/3"),
        ] {
            assert_eq!(get_uri_path_parent(path), want);
        }
    }

    #[test]
    fn test_remove_extra_slashes() {
        for (path, want) in vec![
            // regular paths
            ("/", "/"),
            ("/1/", "/1/"),
            ("/1/2/", "/1/2/"),
            ("/1/2/3/", "/1/2/3/"),
            // paths starting with extra slashes
            ("////////", "/"),
            ("//////1/", "/1/"),
            ("////1/2/", "/1/2/"),
            ("//1/2/3/", "/1/2/3/"),
            // paths ending with extra slashes
            ("/1//////", "/1/"),
            ("/1/2////", "/1/2/"),
            ("/1/2/3//", "/1/2/3/"),
            // paths with random extra slashes
            ("/////1/////", "/1/"),
            ("/1///////2/", "/1/2/"),
            ("//1////2///", "/1/2/"),
            ("//1//2//3//", "/1/2/3/"),
        ] {
            assert_eq!(remove_extra_slashes(path), want);
        }
    }
}
