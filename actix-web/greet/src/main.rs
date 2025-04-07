use actix_web::{App, HttpServer, Responder, get, web};

#[get("/")]
async fn index() -> impl Responder {
    "Hello, world!"
}

#[get("/hi")]
async fn hi() -> impl Responder {
    format!("hi")
}

#[get("/hi/{name}")]
async fn hi_name(name: web::Path<String>) -> impl Responder {
    format!("hi {name}!")
}

#[get("/bye")]
async fn bye() -> impl Responder {
    format!("bye")
}

#[get("/bye/{name}")]
async fn bye_name(name: web::Path<String>) -> impl Responder {
    format!("bye {name}!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(hi)
            .service(hi_name)
            .service(bye)
            .service(bye_name)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
