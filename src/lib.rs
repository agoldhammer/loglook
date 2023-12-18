// use chrono::DateTime;
use anyhow;
use console::style;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::net::IpAddr;
use std::path::PathBuf;
use std::vec::Vec;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use tokio::sync::mpsc;
use tokio::task::JoinSet;

pub mod geo;
pub mod lkup;
pub mod log_entries;

use log_entries::{HostLogs, LogEntry};

fn read_lines(path: &PathBuf) -> Result<io::Lines<BufReader<File>>, Box<dyn Error + 'static>> {
    let file = File::open(path)?;
    return Ok(io::BufReader::new(file).lines());
}

fn logents_to_ips_set(logentries: &Vec<LogEntry>) -> HashSet<IpAddr> {
    let mut ips = HashSet::new();
    for logentry in logentries {
        ips.insert(logentry.ip);
    }
    ips
}

fn logents_to_ips_to_hl_map(logentries: &Vec<LogEntry>) -> HashMap<IpAddr, HostLogs> {
    let mut map_ips_to_hl: HashMap<IpAddr, HostLogs> = HashMap::new();
    for logentry in logentries.iter() {
        if map_ips_to_hl.contains_key(&logentry.ip) {
            // * ip is already in map, so append to existing hl HostLogs
            map_ips_to_hl
                .entry(logentry.ip)
                .and_modify(|hl| hl.log_entries.push(logentry.clone()));
        } else {
            // * this is first in map with this ip
            let mut v = Vec::new();
            v.push(logentry.clone());
            let hl = HostLogs {
                hostname: "".to_string(), // will be filled later
                log_entries: v,
            };
            map_ips_to_hl.insert(logentry.ip, hl);
        }
    }
    map_ips_to_hl
}

pub async fn run(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    // * input stage
    let lines = read_lines(path)?;
    // * process each logline and collect parsed lines into Vec<LogEntry>
    let maybe_logentries: Vec<anyhow::Result<LogEntry>> = lines
        .map(|line| log_entries::LogEntry::try_from(&line.unwrap()))
        .collect();
    let logentries: Vec<LogEntry> = maybe_logentries.into_iter().map(|le| le.unwrap()).collect();
    let le_count = logentries.len();
    println!("Log lines: {le_count}");

    // * end of input stage, resulting in raw logentries

    // * from raw logentries extract set of unique ips and map from ips to HostLogs structs
    let ip_set = logents_to_ips_set(&logentries);
    // * need another ip_set for geolookups
    let ip_set2 = ip_set.clone();
    // * ---------------

    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");
    let n_pb_items = ip_set.len() as u64;
    let pb_rdns = m.add(ProgressBar::new(n_pb_items));
    pb_rdns.set_style(sty.clone());
    pb_rdns.set_message("rdns");
    let pb_geo = m.add(ProgressBar::new(n_pb_items));
    pb_geo.set_style(sty.clone());
    pb_geo.set_message("geodata");

    let mut ips_to_hl_map = logents_to_ips_to_hl_map(&logentries);

    // * create channels to receive rev lkup results
    const CHAN_BUF_SIZE: usize = 256;
    let (tx_rdns, mut rx_rdns) = mpsc::channel(CHAN_BUF_SIZE);

    let mut join_set = JoinSet::new();
    for ip in ip_set {
        let txa = tx_rdns.clone();
        join_set.spawn(async move { lkup::lkup_hostnames(ip, txa).await });
    }

    let (tx_geo, mut rx_geo) = mpsc::channel(CHAN_BUF_SIZE);
    let mut join_set2: JoinSet<()> = JoinSet::new();
    let api_key = geo::read_config();
    for ip in ip_set2 {
        let txa2 = tx_geo.clone();
        let key = api_key.clone();
        join_set2.spawn(async move { geo::geo_lkup(ip, txa2, key).await });
    }

    // * output stuff
    drop(tx_rdns); // have to drop the original channel that has been cloned for each task
    drop(tx_geo);
    while let Some(rev_lookup_data) = rx_rdns.recv().await {
        let ip = rev_lookup_data.ip_addr;
        pb_rdns.inc(1);
        // TODO: only using first of poss several ptr records. FIX!
        let host = &rev_lookup_data.ptr_records[0];
        ips_to_hl_map
            .entry(ip)
            .and_modify(|hl| hl.hostname = host.to_string());
    }

    pb_rdns.finish_and_clear();

    let mut ips_to_geodata_map: HashMap<IpAddr, geo::Geodata> = HashMap::new();
    while let Some(geo_lookup_data) = rx_geo.recv().await {
        pb_geo.inc(1);
        let ip = geo_lookup_data.ip;
        ips_to_geodata_map.insert(ip, geo_lookup_data);
    }

    pb_geo.finish_and_clear();

    for (ip, geodata) in ips_to_geodata_map {
        println!("{}: {}", style("IP").bold().red(), style(ip).green());
        println!("{geodata}");
        let hls = ips_to_hl_map.get(&ip).unwrap();
        let hostlogs = hls.to_owned();
        println!("{hostlogs}");
    }

    println!("Finished processing {le_count} log entries");
    // * end of output stuff

    while let Some(res) = join_set.join_next().await {
        res.expect("all async chans should finish");
    }

    Ok(())
}
