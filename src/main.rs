use clap::{Args, Parser, Subcommand};
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
        // / days back from present time, e.g. 2
        // #[clap(long, short, required_unless_present("start"), conflicts_with("start"))]
        // days: Option<i32>,
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
        country: Option<String>,

        /// search by organization
        #[clap(long, short)]
        org: Option<String>,
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
        Command::Search {
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
            loglook::search(start, end, ip, country, org).await
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
