// mod args;
// use args::Cli;
// use args::{Commands, LoglookArgs};
use clap::{Args, Parser, Subcommand};
use std::process;

// * https://rust-cli-recommendations.sunshowers.io/handling-arguments.html
/// Here's my app!
#[derive(Debug, Parser)]
#[clap(name = "loglook", version = "0.2", about = "Log Reader")]
pub struct App {
    // #[clap(flatten)]
    // global_opts: GlobalOpts,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Read the specified logfile
    Read {
        /// Run as daemon
        #[clap(long, short = 'd')]
        daemon: bool,

        /// The path to read logfile from
        path: std::path::PathBuf,
        // (can #[clap(flatten)] other argument structs here)
    },
    // / Search for data by IP address
    // FindIp(FindIpArgs),
    // ...other commands (can #[clap(flatten)] other enum variants here)
}

#[derive(Debug, Args)]
struct FindIpArgs {
    /// IP address to search for
    ip: String,
    // a list of other write args
}

// #[derive(Debug, Args)]
// struct GlobalOpts {
//     /// Color
//     #[clap(long, arg_enum, global = true, default_value_t = Color::Auto)]
//     color: Color,

//     /// Verbosity level (can be specified multiple times)
//     #[clap(long, short, global = true, parse(from_occurrences))]
//     verbose: usize,
//     //... other global options
// }

// #[derive(Clone, Debug, ArgEnum)]
// enum Color {
//     Always,
//     Auto,
//     Never,
// }

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let cli = App::parse();
    // let args = cli.command
    let result = match &cli.command {
        #[allow(unused_variables)]
        Command::Read { daemon, path } => loglook::run(path).await,
        // Command::FindIp(ip) => nop,
    };

    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("Application error: {}", e);
            process::exit(1);
        }
    }
}
