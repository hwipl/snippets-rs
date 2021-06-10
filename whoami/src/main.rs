// whoami examples based on whoami docs
fn main() {
    println!("whoami::realname():    {}", whoami::realname());
    println!("whoami::username():    {}", whoami::username());
    println!(
        "whoami::lang():        {:?}",
        whoami::lang().collect::<Vec<String>>()
    );
    println!("whoami::devicename():  {}", whoami::devicename());
    println!("whoami::hostname():    {}", whoami::hostname());
    println!("whoami::platform():    {}", whoami::platform());
    println!("whoami::distro():      {}", whoami::distro());
    println!("whoami::desktop_env(): {}", whoami::desktop_env());
}
