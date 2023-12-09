use chrono::DateTime;
use regex::{Captures, Regex};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::net::IpAddr;
use std::path::PathBuf;
use std::process;
use std::vec::Vec;

pub mod ips;

// LogEntry holds info derived from one line of log file
#[allow(dead_code)]
#[derive(Debug)]
struct LogEntry {
    ip: IpAddr,
    time: String,
    method: String,
    code: u32,
    nbytes: u32,
    referrer: String,
    ua: String,
    line: String,
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "decoding {}:\n", self.line)?;
        write!(f, "  ip: {}\n", self.ip)?;
        write!(f, "  time: {}\n", self.time)?;
        write!(f, "  method: {}\n", self.method)?;
        write!(f, "  code: {}\n", self.code)?;
        write!(f, "  nbytes: {}\n", self.nbytes)?;
        write!(f, "  referrer: {}\n", self.referrer)?;
        write!(f, "  user agent: {}\n", self.ua)?;
        write!(f, "end\n")
    }
}

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

fn print_logentries(logentries: &Vec<LogEntry>) {
    for logentry in logentries {
        println!("{}", &logentry);
    }
    println!("Total {} loglines processed\n", logentries.len());
}

pub fn run(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    // regex for parsing nginx log lines in default setup for loal server
    let re = Regex::new(
                    r#"(?<ip>\S+) \S+ \S+ \[(?<time>.+)\] "(?<method>.*)" (?<code>\d+) (?<nbytes>\d+) "(?<referrer>.*)" "(?<ua>.*)""#,
                )
                .unwrap();
    let lines = read_lines(path)?;
    let mut ips = HashSet::new();
    // process each logline and collect parsed lines into Vec<LogEntry>
    let logentries = lines
        .map(|line| make_logentry(&re, line.unwrap()))
        .collect();
    print_logentries(&logentries);
    for logentry in logentries {
        ips.insert(logentry.ip);
    }
    ips::printips(ips);
    return Ok(());
}
