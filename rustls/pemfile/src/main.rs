use rustls_pemfile::{read_one, Item};
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::iter;

fn parse_pem_file(file: String) -> std::io::Result<()> {
    // open file
    let f = File::open(file)?;
    let mut reader = BufReader::new(f);

    // parse file
    for item in iter::from_fn(|| read_one(&mut reader).transpose()) {
        match item.unwrap() {
            Item::X509Certificate(cert) => println!("certificate {cert:?}"),
            Item::RSAKey(key) => println!("rsa pkcs1 key {key:?}"),
            Item::PKCS8Key(key) => println!("pkcs8 key {key:?}"),
            Item::ECKey(key) => println!("sec1 ec key {key:?}"),
            _ => println!("unhandled item"),
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    for file in env::args().skip(1) {
        println!("Parsing file {file}");
        parse_pem_file(file)?;
    }
    Ok(())
}
