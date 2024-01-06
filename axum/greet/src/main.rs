use axum::{extract::Path, routing::get, Router};

async fn index() -> &'static str {
    "Hello, world!"
}

async fn hi() -> &'static str {
    "hi"
}

async fn hi_name(Path(name): Path<String>) -> String {
    format!("hi {name}")
}

async fn bye() -> &'static str {
    "bye"
}

async fn bye_name(Path(name): Path<String>) -> String {
    format!("bye {name}")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(index))
        .route("/hi", get(hi))
        .route("/hi/:name", get(hi_name))
        .route("/bye", get(bye))
        .route("/bye/:name", get(bye_name));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
