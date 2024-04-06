// whoami examples based on whoami docs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("whoami::realname():    {}", whoami::realname());
    println!("whoami::username():    {}", whoami::username());
    println!(
        "whoami::langs():        {:?}",
        whoami::langs()?
            .map(|l| l.to_string())
            .collect::<Vec<String>>()
    );
    println!("whoami::devicename():  {}", whoami::devicename());
    println!(
        "whoami::fallible::hostname():    {}",
        whoami::fallible::hostname()?
    );
    println!("whoami::platform():    {}", whoami::platform());
    println!("whoami::distro():      {}", whoami::distro());
    println!("whoami::desktop_env(): {}", whoami::desktop_env());

    Ok(())
}
