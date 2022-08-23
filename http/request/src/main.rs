use http::{Request, Version};
use std::error::Error;
use std::fmt::Write;

/// head of a request as a string that includes:
/// - the start-line
/// - the headers
/// - the empty line separating the head from the body
struct RequestHead(String);

/// try to get the request head from a request
impl<T> TryFrom<&Request<T>> for RequestHead {
    type Error = Box<dyn Error>;

    fn try_from(request: &Request<T>) -> Result<Self, Self::Error> {
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

        Ok(RequestHead(s))
    }
}

/// convert the request head to string
impl From<RequestHead> for String {
    fn from(head: RequestHead) -> String {
        head.0
    }
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

    let head = String::from(RequestHead::try_from(&request)?);
    println!("head as string:\n{}", head);
    println!("head as bytes:\n{:?}", head.as_bytes());

    Ok(())
}
