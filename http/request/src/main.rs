use http::{Request, Version};
use std::error::Error;
use std::fmt::Write;

/// get the head of the request as a string, this includes:
/// - the start-line
/// - the headers
/// - the empty line separating the head from the body
fn get_head_string<T>(request: &Request<T>) -> Result<String, Box<dyn Error>> {
    let mut s = String::new();

    // write start-line
    write!(
        &mut s,
        "{} {} {:?}\r\n",
        request.method(),
        request.uri(),
        request.version()
    )?;

    // write headers
    for (name, value) in request.headers() {
        write!(&mut s, "{}: {}\r\n", name, value.to_str()?)?;
    }

    // write empty line
    write!(&mut s, "\r\n")?;

    Ok(s)
}

fn main() -> Result<(), Box<dyn Error>> {
    let request = Request::builder()
        .method("GET")
        .uri("/")
        .version(Version::HTTP_11)
        .header("Host", "www.rust-lang.org")
        .header("Connection", "close")
        .header("Accept-Encoding", "identity")
        .body(())?;
    let head = get_head_string(&request)?;
    println!("head as string:\n{}", head);
    println!("head as bytes:\n{:?}", head.as_bytes());

    Ok(())
}
