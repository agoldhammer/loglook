use clap::{ArgAction, Parser, Subcommand};
use ctrlc;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

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

        /// regex search by IP address
        #[clap(long, short, group = "select")]
        ip: Option<String>,

        /// regex search by country
        #[clap(long, short, group = "select")]
        #[arg(num_args(0..))]
        country: Option<Vec<String>>,

        /// regex search by organization
        #[clap(long, short, group = "select")]
        org: Option<String>,
    },
}

async fn read(daemon: &bool, path: &PathBuf) -> anyhow::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    while running.load(Ordering::SeqCst) {
        println!("Running...");
        loglook::run(daemon, path).await?;
        // * read every 30 minutes
        // TODO add variable duration???
        if *daemon {
            sleep(Duration::from_secs(1800)).await;
        } else {
            break;
        }
    }

    println!("Exiting gracefully!");
    Ok(())
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let cli = App::parse();
    // let args = cli.command
    let result = match &cli.command {
        #[allow(unused_variables)]
        Command::Read { daemon, path } => read(daemon, path).await,
        Command::Search {
            nologs,
            start,
            end,
            ip,
            country,
            org,
        } => loglook::search(nologs, start, end, ip, country, org).await,
    };

    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("Application error: {}", e);
            process::exit(1);
        }
    }
}
