mod args;
use args::LoglookArgs;
use clap::Parser;
use std::process;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let args = LoglookArgs::parse();
    println!("Opening file: {:?}", args.path);
    println!("Show {}", args.show);
    let result = loglook::run(&args.path).await;
    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("Application error: {}", e);
            process::exit(1);
        }
    }
}
