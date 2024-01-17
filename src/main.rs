use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// * https://rust-cli-recommendations.sunshowers.io/handling-arguments.html
#[derive(Debug, Parser)]
#[clap(name = "loglook", version = "0.3", about = "Log Reader")]
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

async fn read(daemon: &bool, path: &PathBuf, config: &loglook::Config) -> anyhow::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    // * read every 30 minutes
    // TODO add variable duration???
    // * wake up once a second to check for ctrl-c; rerun main fn when wait reduced to 0
    let mut seconds_till_run = 0;
    while running.load(Ordering::SeqCst) {
        if seconds_till_run == 0 {
            seconds_till_run = 1800; // reset to 30 minutes
            loglook::read(daemon, path, config).await?;
        }

        if *daemon {
            sleep(Duration::from_secs(1)).await;
            seconds_till_run -= 1;
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
    let config = loglook::read_config();
    // let args = cli.command
    let result = match &cli.command {
        #[allow(unused_variables)]
        Command::Read { daemon, path } => read(daemon, path, &config).await,
        Command::Search {
            nologs,
            start,
            end,
            ip,
            country,
            org,
        } => loglook::search(nologs, start, end, ip, country, org, &config).await,
    };

    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("Application error: {}", e);
            process::exit(1);
        }
    }
}
