use rcgen::generate_simple_self_signed;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // set subject alternative names
    let subject_alt_names: Vec<String> = ["example.com", "localhost", "other.example.com"]
        .map(String::from)
        .to_vec();

    // create certificate
    let cert = generate_simple_self_signed(subject_alt_names)?;
    println!("{}", cert.serialize_pem()?);
    println!("{}", cert.serialize_private_key_pem());

    Ok(())
}
