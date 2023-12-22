use console::style;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines};
use std::net::IpAddr;
use std::path::PathBuf;
use std::vec::Vec;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use config_file::FromConfigFile;
// use mongodb::bson::{doc, to_document};
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

pub mod geo;
pub mod lkup;
pub mod log_entries;

use log_entries::LogEntry;

use crate::lkup::RevLookupData;

#[derive(Deserialize)]
#[allow(dead_code)]
struct Config {
    api_key: String,
    db_uri: String,
}

#[derive(Serialize, Deserialize)]
struct HostData {
    geodata: geo::Geodata,
    ptr_records: Vec<String>,
}

impl fmt::Display for HostData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{}: {}",
            style("IP").bold().red(),
            style(self.geodata.ip).green()
        )
        .unwrap();

        write!(f, "{}", self.geodata).unwrap();
        self.ptr_records.iter().try_for_each(|record| {
            writeln!(f, "{}: {}", style("host").red(), style(record).green())
        })
    }
}

fn read_config() -> Config {
    let path = shellexpand::tilde("~/.loglook/config.toml");
    let config = Config::from_config_file(path.as_ref()).unwrap();
    config
}

fn read_lines(path: &PathBuf) -> Result<io::Lines<BufReader<File>>, Box<dyn Error + 'static>> {
    let file = File::open(path)?;
    Ok(io::BufReader::new(file).lines())
}

fn logents_to_ips_set(logentries: &[LogEntry]) -> HashSet<IpAddr> {
    let mut ips = HashSet::new();
    for logentry in logentries {
        ips.insert(logentry.ip);
    }
    ips
}

fn make_logentries(lines: Lines<BufReader<File>>) -> Vec<LogEntry> {
    let maybe_logentries: Vec<anyhow::Result<LogEntry>> = lines
        .map(|line| log_entries::LogEntry::try_from(&line.expect("log line should be readable")))
        .collect();
    // * deal with errors (poss bad lines in log) here by displaying on stderr
    let mut logentries: Vec<LogEntry> = Vec::new();
    for maybe_logentry in maybe_logentries.into_iter() {
        match maybe_logentry {
            Ok(logentry) => logentries.push(logentry),
            Err(e) => eprintln!("Log read error: {}", e),
        }
    }
    logentries
}

fn progress_bar_setup(n_pb_items: u64) -> (ProgressBar, ProgressBar) {
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");
    // let n_pb_items = ip_set.len() as u64;
    let pb_rdns = m.add(ProgressBar::new(n_pb_items));
    pb_rdns.set_style(sty.clone());
    pb_rdns.set_message("rdns");
    let pb_geo = m.add(ProgressBar::new(n_pb_items));
    pb_geo.set_style(sty.clone());
    pb_geo.set_message("geodata");
    (pb_rdns, pb_geo)
}

async fn setup_db(
    config: &Config,
) -> Result<(Collection<HostData>, Collection<LogEntry>), Box<dyn Error>> {
    let client = Client::with_uri_str(&config.db_uri).await?;
    // dbg!(client);
    // TODO: should take dbname from config
    let db = client.database("actulogs");
    let host_data_coll: Collection<HostData> = db.collection("hostdata");
    let logents_coll: Collection<LogEntry> = db.collection("logentries");
    (host_data_coll, logents_coll)
}

pub async fn run(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    /* Strategy: Parse loglines into LogEntries
    Do reverse dns lookup to generate RevLookupData, collect in map with ip as key
    Do geo lookup to generate Geodata, collect in map with ip as key
    Output to console
    Eventually, send to mongo db
     */
    let config = read_config();
    // * setup database
    let (host_data_coll, logents_coll) = setup_db(&config).await?;
    // * input stage
    let lines = read_lines(path)?;
    // * process each logline and collect parsed lines into Vec<LogEntry>
    let logentries = make_logentries(lines);
    let le_count = logentries.len();
    println!("Log lines: {le_count}");

    // * end of input stage, resulting in raw logentries

    // * from raw logentries extract set of unique ips and map from ips
    let ip_set = logents_to_ips_set(&logentries);
    // * need another ip_set for geolookups
    let ip_set2 = ip_set.clone();
    // * ---------------
    let (pb_rdns, pb_geo) = progress_bar_setup(ip_set.len() as u64);

    let mut ips_to_rdns_map: HashMap<IpAddr, RevLookupData> = HashMap::new();

    // * create channels to receive rev lkup results
    const CHAN_BUF_SIZE: usize = 256;
    let (tx_rdns, mut rx_rdns) = mpsc::channel(CHAN_BUF_SIZE);

    let mut join_set = JoinSet::new();
    for ip in ip_set {
        let txa = tx_rdns.clone();
        join_set.spawn(async move { lkup::lkup_hostnames(ip, txa).await });
    }

    let (tx_geo, mut rx_geo) = mpsc::channel(CHAN_BUF_SIZE);
    for ip in ip_set2 {
        let txa2 = tx_geo.clone();
        let key = config.api_key.clone();
        join_set.spawn(async move { geo::geo_lkup(ip, txa2, key).await });
    }

    // * output stuff
    drop(tx_rdns); // have to drop the original channel that has been cloned for each task
    drop(tx_geo);

    let mut ips_to_geodata_map: HashMap<IpAddr, geo::Geodata> = HashMap::new();
    while let Some(geo_lookup_data) = rx_geo.recv().await {
        pb_geo.inc(1);
        let ip = geo_lookup_data.ip;
        ips_to_geodata_map.insert(ip, geo_lookup_data);
    }

    while let Some(rev_lookup_data) = rx_rdns.recv().await {
        let ip = rev_lookup_data.ip_addr;
        pb_rdns.inc(1);
        ips_to_rdns_map.insert(ip, rev_lookup_data);
    }

    pb_rdns.finish();
    pb_geo.finish();

    let mut ip_to_hostdata_map = HashMap::new();
    println!("\nOutput");
    for (ip, geodata) in ips_to_geodata_map {
        println!("{}: {}", style("IP").bold().red(), style(ip).green());
        println!("{geodata}");
        let rdns = ips_to_rdns_map.get(&ip).unwrap();
        let hostdata = HostData {
            geodata,
            ptr_records: rdns.ptr_records.clone(),
        };
        ip_to_hostdata_map.insert(ip, hostdata);
        println!("{rdns}\n");
        let les = logentries.iter().filter(|le| le.ip == ip);
        for le in les {
            println!("{le}");
        }
    }

    println!("Hostdata");
    for hd in ip_to_hostdata_map.values() {
        println!("{hd}");
    }

    let docs = ip_to_hostdata_map.values();
    host_data_coll.insert_many(docs, None).await?;
    logents_coll.insert_many(logentries, None).await?;

    println!("Finished processing {le_count} log entries");
    // * end of output stuff

    while let Some(res) = join_set.join_next().await {
        res.expect("all async chans should finish");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // use super::*;
    use crate::read_config;

    #[test]
    fn config_read_rest() {
        let db_uri = read_config().db_uri;
        assert!(db_uri.contains("27017"));
    }
}
