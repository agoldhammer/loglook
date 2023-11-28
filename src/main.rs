use clap::Parser;
use std::process;

#[derive(Parser)]
struct Cli {
    path: std::path::PathBuf,
}

fn main() {
    // let ip_str = "162.243.141.14";
    // let ip = ip_str.parse::<IpAddr>().expect("should have good ip addr");
    // let host = lookup_addr(&ip).unwrap();
    // println!("host: {}", host);
    let args = Cli::parse();
    println!("Opening file: {:?}", args.path);
    // process_logfile(&args.path);
    if let Err(e) = loglook::run(&args.path) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}
