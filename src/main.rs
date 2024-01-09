use clap::{ArgAction, Parser, Subcommand};
use std::process;

// * https://rust-cli-recommendations.sunshowers.io/handling-arguments.html
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
    /// Find ips in date range
    Search {
        #[clap(long="no-logs", short, action=ArgAction::SetTrue)]
        /// no output of logentries
        nologs: Option<bool>,

        /// start time, e.g. ISO: 2023-12-29T00:00:00Z
        #[clap(long, short)]
        start: String,

        /// end time e.g. ISO: 2023-12-29T00:00:00Z
        #[clap(long, short)]
        end: String,

        /// search for IP address
        #[clap(long, short)]
        ip: Option<String>,

        /// search by country
        #[clap(long, short)]
        #[arg(num_args(0..))]
        country: Option<Vec<String>>,

        /// search by organization
        #[clap(long, short)]
        org: Option<String>,
    },
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let cli = App::parse();
    // let args = cli.command
    let result = match &cli.command {
        #[allow(unused_variables)]
        Command::Read { daemon, path } => loglook::run(daemon, path).await,
        Command::Search {
            nologs,
            start,
            end,
            ip,
            country,
            org,
        } => {
            println!(
                "s {:?}, e {:?}, ip {:?} co {:?} org {:?}",
                start, end, ip, country, org
            );
            loglook::search(nologs, start, end, ip, country, org).await
        }
    };

    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("Application error: {}", e);
            process::exit(1);
        }
    }
}
