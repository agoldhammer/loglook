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

pub mod ips;
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

fn ips_to_hashset(logentries: &Vec<LogEntry>) -> HashSet<IpAddr> {
    let mut ips = HashSet::new();
    for logentry in logentries {
        ips.insert(logentry.ip);
    }
    ips
}

pub fn run(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    // regex for parsing nginx log lines in default setup for loal server
    let re = Regex::new(
                    r#"(?<ip>\S+) \S+ \S+ \[(?<time>.+)\] "(?<method>.*)" (?<code>\d+) (?<nbytes>\d+) "(?<referrer>.*)" "(?<ua>.*)""#,
                )
                .unwrap();
    let lines = read_lines(path)?;
    // * process each logline and collect parsed lines into Vec<LogEntry>
    let logentries: Vec<LogEntry> = lines
        .map(|line| make_logentry(&re, line.unwrap()))
        .collect();

    let ip_set = ips_to_hashset(&logentries);

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
            map_ips_to_hl.insert(logentry.ip.clone(), hl);
        }
    }

    for (ip, hl) in map_ips_to_hl {
        println!("{}: {}", style("IP").bold().red(), style(ip).green());
        println!("{hl}");
        println! {"{}\n", style("_".repeat(80)).cyan().bright()};
    }

    ips::printips(&ip_set);

    return Ok(());
}
