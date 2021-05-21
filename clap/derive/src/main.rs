use clap::{AppSettings, Clap};

const VERSION: &str = "0.1.0";

/// Tests clap command line argument parsing
#[derive(Clap)]
#[clap(version = VERSION)]
#[clap(setting = AppSettings::ColoredHelp)]
struct Args {
    /// Greeting
    #[clap(short, long, default_value = "hi")]
    greeting: String,
    /// Persons to greet
    #[clap(short, long)]
    persons: Vec<String>,
}

fn main() {
    let args = Args::parse();

    for p in args.persons {
        println!("{} {}", args.greeting, p);
    }
}
