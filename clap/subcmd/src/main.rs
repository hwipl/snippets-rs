use clap::{AppSettings, Clap};

/// Tests clap command line argument parsing with subcommands
#[derive(Clap)]
#[clap(version)]
#[clap(setting = AppSettings::ColoredHelp)]
struct Args {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Say "Hi" to persons
    Hi(Names),
    /// Say "Bye" to persons
    Bye(Names),
}

#[derive(Clap)]
struct Names {
    /// Names of persons
    #[clap(name = "NAME")]
    names: Vec<String>,
}

fn main() {
    let args = Args::parse();

    match args.subcmd {
        SubCommand::Hi(names) => {
            for n in names.names {
                println!("Hi, {}!", n);
            }
        }
        SubCommand::Bye(names) => {
            for n in names.names {
                println!("Bye, {}!", n);
            }
        }
    }
}
