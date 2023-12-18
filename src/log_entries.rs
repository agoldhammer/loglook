// LogEntry holds info derived from one line of log file
use chrono::DateTime;
use console::style;
// use serde::de::Error;
use core::convert::TryFrom;
use regex::{Captures, Regex};
use std::fmt;
use std::net::IpAddr;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub ip: IpAddr,
    pub time: String,
    pub method: String,
    pub code: u32,
    pub nbytes: u32,
    pub referrer: String,
    pub ua: String,
    pub line: String,
}

// * type to hold both a hostname and a vector of
// * LogEntry types representing activity on that host
// * will be collected in a hash map
// * called map_ips_to_logents with ip as index
#[derive(Debug, Clone)]
pub struct HostLogs {
    pub hostname: String,
    pub log_entries: Vec<LogEntry>,
}

impl fmt::Display for HostLogs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {}\n---\n",
            style("Hostname").red().bold(),
            style(&self.hostname).green()
        )
        .expect("shd wrt ok");
        for log_entry in self.log_entries.iter() {
            write!(f, "{}\n", log_entry).expect("shd wrt ok");
        }
        write!(f, "{}", style("-".repeat(40)).cyan())
    }
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // write!(f, "  ip: {}\n", self.ip)?;
        write!(f, "  time: {}\n", self.time)?;
        write!(f, "  method: {}\n", self.method)?;
        write!(f, "  code: {}\n", self.code)?;
        write!(f, "  nbytes: {}\n", self.nbytes)?;
        write!(f, "  referrer: {}\n", self.referrer)?;
        write!(f, "  user agent: {}\n", self.ua)?;
        write!(f, "  logged: {}:\n", self.line)?;
        write!(f, "end\n")
    }
}

fn get_re_match_part(caps: &Captures<'_>, part_name: &str) -> String {
    let part = caps.name(part_name).unwrap().as_str();
    return String::from(part);
}

impl TryFrom<&String> for LogEntry {
    type Error = String;

    fn try_from(line: &String) -> Result<Self, Self::Error> {
        let re = Regex::new(
                    r#"(?<ip>\S+) \S+ \S+ \[(?<time>.+)\] "(?<method>.*)" (?<code>\d+) (?<nbytes>\d+) "(?<referrer>.*)" "(?<ua>.*)""#,
                )
                .unwrap();
        let caps = match re.captures(line) {
            Some(x) => x,
            None => {
                let s = format!("Failed to parse line: {:?}", line);
                return Err(s.clone());
            }
        };
        let ip_str = get_re_match_part(&caps, "ip");
        let ip = ip_str.parse::<IpAddr>().expect("should have good ip addr");
        let code_str = get_re_match_part(&caps, "code");
        let nbytes_str = get_re_match_part(&caps, "nbytes");
        let time_str = get_re_match_part(&caps, "time");
        let time = DateTime::parse_from_str(time_str.as_str(), "%d/%b/%Y:%H:%M:%S %z")
            .expect("should be valid time fmt");
        let le = LogEntry {
            ip: ip,
            time: time.to_string(),
            method: get_re_match_part(&caps, "method"),
            code: code_str.parse().unwrap(),
            nbytes: nbytes_str.parse().unwrap(),
            referrer: get_re_match_part(&caps, "referrer"),
            ua: get_re_match_part(&caps, "ua"),
            line: line.to_owned(),
        };
        Ok(le)
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn logline_to_logent_conv_test() {
        let line = "180.149.125.164 - - [25/Nov/2023:00:16:58 -0500] \"GET /stalker_portal/server/tools/auth_simple.php HTTP/1.1\" 404 209 \"-\" \"Mozilla/5.0 (Windows NT 5.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36\"".to_string();
        let le = LogEntry::try_from(&line).unwrap();
        assert_eq!(le.line, line);
    }

    #[test]
    fn detect_bad_line_test() {
        let line = "180.149.125.164 - - 25/Nov/2023:00:16:58 -0500] \"GET /stalker_portal/server/tools/auth_simple.php HTTP/1.1\" 404 209 \"-\" \"Mozilla/5.0 (Windows NT 5.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36\"".to_string();
        assert!(LogEntry::try_from(&line).is_err());
    }
}
