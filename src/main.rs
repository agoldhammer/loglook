// use std::error::Error;
use regex::{Captures, Regex};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::process;
use std::net::IpAddr;
use chrono::DateTime;
use std::vec::Vec;
// use chrono::format::ParseError;

#[allow(dead_code)]
#[derive(Debug)]
struct LogEntry {
    ip: IpAddr,
    time: String,
    method: String,
    code: u32,
    bytes: u32,
    misc: String,
    ua: String,
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn get_re_match_part(caps: &Captures<'_>, part_name: &str) -> String {
    let part = caps.name(part_name).unwrap().as_str();
    return String::from(part);
}

// Convert line to logentry
fn make_logentry(re: &Regex, line: String) -> LogEntry {
    let caps = re.captures(&line).unwrap();
    let ip_str = get_re_match_part(&caps, "ip");
    let ip = ip_str.parse::<IpAddr>().expect("should have good ip addr");
    let code_str = get_re_match_part(&caps, "code");
    let bytes_str = get_re_match_part(&caps, "bytes");
    let time_str = get_re_match_part(&caps, "time");
    let time = DateTime::parse_from_str(time_str.as_str(), "%d/%b/%Y:%H:%M:%S %z").expect("should be valid time fmt");
    return LogEntry {
        ip: ip,
        time: time.to_string(),
        method: get_re_match_part(&caps, "method"),
        code: code_str.parse().unwrap(),
        bytes: bytes_str.parse().unwrap(),
        misc: get_re_match_part(&caps, "misc"),
        ua: get_re_match_part(&caps, "ua"),
    };
}

fn main() {
    // regex for parsing nginx log lines in default setup for loal server
    let logfname = "./access.log";
    let re = Regex::new(
                    r#"(?<ip>\S+) - - \[(?<time>.+)\] "(?<method>.+)" (?<code>\d+) (?<bytes>\d+) "(?<misc>.+)" "(?<ua>.+)""#,
                )
                .unwrap();
    let mut logentries: Vec<LogEntry> = Vec::new();
    if let Ok(lines) = read_lines(logfname) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(line) = line {
                logentries.push(make_logentry(&re, line));
            }
        }
        for logentry in &logentries {
            dbg!(logentry);
            println!("\n");
        }
        println!("No. entries: {}", logentries.len())
    } else {
        println!("Error WTF?");
        process::exit(1);
    }
}
