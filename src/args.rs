#[allow(unused_imports)]
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
// #[clap(Version, About)]
pub struct LoglookArgs {
    /// show stuff
    #[clap(long, short, action)]
    pub show: bool,
    /// path to log file
    pub path: std::path::PathBuf,
}
