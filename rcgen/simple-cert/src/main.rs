use rcgen::generate_simple_self_signed;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // set subject alternative names
    let subject_alt_names: Vec<String> = ["example.com", "localhost", "other.example.com"]
        .map(String::from)
        .to_vec();

    // create certificate
    let cert = generate_simple_self_signed(subject_alt_names)?;
    println!("{}", cert.cert.pem());
    println!("{}", cert.key_pair.serialize_pem());

    Ok(())
}
