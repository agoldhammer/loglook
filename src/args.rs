#[allow(unused_imports)]
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version = "0.2", about = "Log Reader")]
pub struct Cli {
    /// show stuff
    // #[arg(long, short)]
    // pub show: bool,
    #[command(subcommand)]
    pub command: Commands,
    // path to log file
    // pub path: std::path::PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// read log file from path
    Read(ReadArgs),
    /// specify end time of query
    FindTime(TimeArgs),
    /// search by ip
    FindIp(FindIpArgs),
    /// search by organization
    FindOrg(FindOrgArgs),
    /// search by country
    FindCountry(FindCountryArgs),
}

#[derive(Args, Debug)]
pub struct ReadArgs {
    pub path: std::path::PathBuf,
}

#[derive(Args, Debug)]
pub struct TimeArgs {
    end: Option<String>,
}

#[derive(Args, Debug)]
pub struct FindCountryArgs {
    country: String,
}

#[derive(Args, Debug)]
pub struct FindOrgArgs {
    org: String,
}

#[derive(Args, Debug)]
pub struct FindIpArgs {
    ip: String,
}

#[derive(Parser, Debug)]
pub enum LoglookArgs {
    ReadArgs,
    TimeArgs,
    FindCountryArgs,
    FindIpArgs,
}
