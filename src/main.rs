// use std::error::Error;
use regex::{Captures, Regex};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::process;

#[allow(dead_code)]
#[derive(Debug)]
struct LogEntry {
    ip: String,
    time: String,
    method: String,
    code: String,
    bytes: String,
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
fn make_logentry(line: String) {
    let re = Regex::new(
                    r#"(?<ip>\S+) - - \[(?<time>.+)\] (?<method>".+")(?<code>\W\d+\W)(?<bytes>\d+) "(?<misc>.+)" "(?<ua>.+)""#,
                )
                .unwrap();
    let caps = re.captures(&line).unwrap();
    let logentry = LogEntry {
        ip: get_re_match_part(&caps, "ip"),
        time: get_re_match_part(&caps, "time"),
        method: get_re_match_part(&caps, "method"),
        code: get_re_match_part(&caps, "code"),
        bytes: get_re_match_part(&caps, "bytes"),
        misc: get_re_match_part(&caps, "misc"),
        ua: get_re_match_part(&caps, "ua"),
    };
    // println!("Logentry ip {}", logentry.ip);
    dbg!(logentry);
    println!("line: {}", line);
    println!("\n");
}

fn main() {
    // File hosts.txt must exist in the current path
    if let Ok(lines) = read_lines("./access.log") {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(line) = line {
                make_logentry(line);
                // let re = Regex::new(
                //     r#"(?<ip>\d+\.\d+\.\d+\.\d+)\W-\W-\W\[(?<time>.+)\] (?<method>".+")(?<code>\W\d+\W)(?<bytes>\d+) "(?<misc>.+)" "(?<ua>.+)""#,
                // )
                // let re = Regex::new(
                //     r#"(?<ip>\S+) - - \[(?<time>.+)\] (?<method>".+")(?<code>\W\d+\W)(?<bytes>\d+) "(?<misc>.+)" "(?<ua>.+)""#,
                // )
                // .unwrap();
                // let m = re.find(&line).unwrap();
                // let caps = re.captures(&line).unwrap();
                // println!("\n");
                // println!("IP: {}", caps.name("ip").unwrap().as_str());
                // println!("Time: {}", caps.name("time").unwrap().as_str());
                // println!("Method: {}", caps.name("method").unwrap().as_str());
                // println!("Code: {}", caps.name("code").unwrap().as_str());
                // println!("Bytes: {}", caps.name("bytes").unwrap().as_str());
                // println!("Misc: {}", caps.name("misc").unwrap().as_str());
                // println!("UA: {}", caps.name("ua").unwrap().as_str());
                // println!("m: {}", m.as_str());
                // println!("line: {}", line);
                // println!("\n");
            }
        }
    } else {
        println!("Error WTF?");
        process::exit(1)
    }
}
