use chrono::DateTime;
use regex::{Captures, Regex};
use std::collections::{HashMap, HashSet};
use std::error::Error;
// use std::fmt;
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

// Convert line to logentry
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

pub fn run(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    // regex for parsing nginx log lines in default setup for loal server
    let re = Regex::new(
                    r#"(?<ip>\S+) \S+ \S+ \[(?<time>.+)\] "(?<method>.*)" (?<code>\d+) (?<nbytes>\d+) "(?<referrer>.*)" "(?<ua>.*)""#,
                )
                .unwrap();
    let lines = read_lines(path)?;
    let mut ips = HashSet::new();
    // * process each logline and collect parsed lines into Vec<LogEntry>
    let logentries: Vec<LogEntry> = lines
        .map(|line| make_logentry(&re, line.unwrap()))
        .collect();
    for logentry in &logentries {
        ips.insert(logentry.ip.clone());
    }

    // * Added new stuff
    let mut ips2logentries = HashMap::new();
    for ip in ips.iter() {
        let mut v = Vec::new();
        for le in logentries.clone() {
            if le.ip == *ip {
                v.push(le);
            }
        }
        let hl = HostLogs {
            hostname: "".to_string(),
            log_entries: v,
        };
        ips2logentries.insert(ip, hl);
    }
    for (ip, hls) in ips2logentries {
        println!("IP: {ip}----------");
        println!("Log Entry: {hls}");
        println! {"===================="};
        // dbg!(hls);
    }

    // * end of  new stuff
    ips::printips(&ips);

    return Ok(());
}
