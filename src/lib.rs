use chrono::DateTime;
use console::style;
use regex::{Captures, Regex};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::net::IpAddr;
use std::path::PathBuf;
use std::process;
use std::vec::Vec;

use indicatif::ProgressBar;

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

fn get_re_match_part(caps: &Captures<'_>, part_name: &str) -> String {
    let part = caps.name(part_name).unwrap().as_str();
    return String::from(part);
}

// * Convert raw log line to logentry using supplied regex to parse the line
fn make_logentry(re: &Regex, line: String) -> LogEntry {
    let caps = match re.captures(&line) {
        Some(x) => x,
        None => {
            eprintln!("Failed to parse line: {:?}", line);
            process::exit(255);
        }
    };
    let ip_str = get_re_match_part(&caps, "ip");
    let ip = ip_str.parse::<IpAddr>().expect("should have good ip addr");
    let code_str = get_re_match_part(&caps, "code");
    let nbytes_str = get_re_match_part(&caps, "nbytes");
    let time_str = get_re_match_part(&caps, "time");
    let time = DateTime::parse_from_str(time_str.as_str(), "%d/%b/%Y:%H:%M:%S %z")
        .expect("should be valid time fmt");
    return LogEntry {
        ip: ip,
        time: time.to_string(),
        method: get_re_match_part(&caps, "method"),
        code: code_str.parse().unwrap(),
        nbytes: nbytes_str.parse().unwrap(),
        referrer: get_re_match_part(&caps, "referrer"),
        ua: get_re_match_part(&caps, "ua"),
        line: line,
    };
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
    // * regex for parsing nginx log lines in default setup for loal server
    let re = Regex::new(
                    r#"(?<ip>\S+) \S+ \S+ \[(?<time>.+)\] "(?<method>.*)" (?<code>\d+) (?<nbytes>\d+) "(?<referrer>.*)" "(?<ua>.*)""#,
                )
                .unwrap();
    let lines = read_lines(path)?;
    // * process each logline and collect parsed lines into Vec<LogEntry>
    let logentries: Vec<LogEntry> = lines
        .map(|line| make_logentry(&re, line.unwrap()))
        .collect();
    let le_count = logentries.len();
    println!("Log lines: {le_count}");
    // let _res = geo::geo_lkup().await;

    // * end of input stage, resulting in raw logentries

    // * from raw logentries extract set of unique ips and map from ips to HostLogs structs
    let ip_set = logents_to_ips_set(&logentries);
    // * need another ip_set for geolookups
    let ip_set2 = ip_set.clone();
    // * ---------------

    let pb = ProgressBar::new(ip_set.len() as u64);

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
    for ip in ip_set2 {
        let txa2 = tx_geo.clone();
        join_set2.spawn(async move { geo::geo_lkup(ip, txa2).await });
    }

    // * output stuff
    // TODO: add channels to receive 2 outputs and pass on to modify hostlogs
    drop(tx_rdns); // have to drop the original channel that has been cloned for each task
    drop(tx_geo);
    while let Some(rev_lookup_data) = rx_rdns.recv().await {
        let ip = rev_lookup_data.ip_addr;
        pb.inc(1);
        // TODO: only using first of poss several ptr records. FIX!
        let host = &rev_lookup_data.ptr_records[0];
        ips_to_hl_map
            .entry(ip)
            .and_modify(|hl| hl.hostname = host.to_string());
    }

    pb.finish_and_clear();

    let mut ips_to_geodata_map: HashMap<IpAddr, geo::Geodata> = HashMap::new();
    while let Some(geo_lookup_data) = rx_geo.recv().await {
        let ip = geo_lookup_data.ip;
        ips_to_geodata_map.insert(ip, geo_lookup_data);
        // println!("{}", geo_lookup_data);
    }

    for (ip, geodata) in ips_to_geodata_map {
        println!("{}: {}", style("IP").bold().red(), style(ip).green());
        println!("{geodata}");
        let hls = ips_to_hl_map.get(&ip).unwrap();
        let hostlogs = hls.to_owned();
        println!("Hostname: {}", hostlogs.hostname);
        for le in hostlogs.log_entries {
            println!("{le}");
            println! {"{}\n", style("_".repeat(80)).cyan().bright()};
        }
    }

    // for (ip, hl) in ips_to_hl_map {
    //     println!("{hl}");
    //     println! {"{}\n", style("_".repeat(80)).cyan().bright()};
    // }

    println!("Finished processing {le_count} log entries");

    // ips::printips(&ip_set);
    // * end of output stuff

    while let Some(res) = join_set.join_next().await {
        res.expect("all async chans should finish");
    }

    return Ok(());
}
