#[macro_use]
extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/hi")]
fn hi() -> &'static str {
    "hi"
}

#[get("/hi/<name>")]
fn hi_name(name: &str) -> String {
    format!("hi {}", name)
}

#[get("/bye")]
fn bye() -> &'static str {
    "bye"
}

#[get("/bye/<name>")]
fn bye_name(name: &str) -> String {
    format!("bye {}", name)
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, hi, hi_name, bye, bye_name])
}
