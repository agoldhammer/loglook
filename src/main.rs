use clap::Parser;
use std::process;

#[derive(Parser)]
struct Cli {
    path: std::path::PathBuf,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let args = Cli::parse();
    println!("Opening file: {:?}", args.path);
    let result = loglook::run(&args.path).await;
    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("Application error: {}", e);
            process::exit(1);
        }
    }
}
