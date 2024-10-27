use anyhow::{Context, Result};
use bson;
use chrono;
use core::convert::TryFrom;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::fmt;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub ip: String,
    pub time: bson::DateTime,
    pub method: String,
    pub code: u32,
    pub nbytes: u32,
    pub referrer: String,
    pub ua: String,
    pub line: String,
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // write!(f, "  ip: {}\n", self.ip)?;
        writeln!(f, "  time: {}", self.time)?;
        writeln!(f, "  method: {}", self.method)?;
        writeln!(f, "  code: {}", self.code)?;
        writeln!(f, "  nbytes: {}", self.nbytes)?;
        writeln!(f, "  referrer: {}", self.referrer)?;
        writeln!(f, "  user agent: {}", self.ua)?;
        writeln!(f, "  logged: {}:", self.line)?;
        writeln!(f, "end")
    }
}

fn get_re_match_part(caps: &Captures<'_>, part_name: &str) -> String {
    let part = caps.name(part_name).unwrap().as_str();
    String::from(part)
}

impl TryFrom<&String> for LogEntry {
    type Error = anyhow::Error;

    fn try_from(line: &String) -> Result<Self> {
        // 10/27/2024: changed regex because identification of $remote_user was incorrect
        // causing some log entries to be skipped
        let re = Regex::new(
                    r#"(?<ip>\S+) - (?<remote_user>[^\[]+) \[(?<time>.+)\] "(?<method>.*)" (?<code>\d+) (?<nbytes>\d+) "(?<referrer>.*)" "(?<ua>.*)""#,

                    // r#"(?<ip>\S+) - \S+ \[(?<time>.+)\] "(?<method>.*)" (?<code>\d+) (?<nbytes>\d+) "(?<referrer>.*)" "(?<ua>.*)""#,
                )
                .unwrap();
        let caps = re
            .captures(line)
            .with_context(|| format!("Failed to parse line: {:?}", line))?;
        let ip_str = get_re_match_part(&caps, "ip");
        let _remote_user = get_re_match_part(&caps, "remote_user");
        let code_str = get_re_match_part(&caps, "code");
        let nbytes_str = get_re_match_part(&caps, "nbytes");
        let time_str = get_re_match_part(&caps, "time");
        let ct_time_fixed =
            chrono::DateTime::parse_from_str(time_str.as_str(), "%d/%b/%Y:%H:%M:%S %z")
                .expect("should be valid time fmt");
        let ct_utc: chrono::DateTime<chrono::Utc> = ct_time_fixed.into();
        let le = LogEntry {
            ip: ip_str.to_string(),
            time: ct_utc.into(),
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
        assert_eq!(le.code, 404);
    }

    #[test]
    fn detect_bad_line_test() {
        // removing open bracket on date
        let line = "180.149.125.164 - - 25/Nov/2023:00:16:58 -0500] \"GET /stalker_portal/server/tools/auth_simple.php HTTP/1.1\" 404 209 \"-\" \"Mozilla/5.0 (Windows NT 5.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36\"".to_string();
        assert!(LogEntry::try_from(&line).is_err());
    }

    #[test]
    fn detect_weird_remote_user_test() {
        // removing open bracket on date
        let line = "158.220.106.204 - goolicker', '', (SELECT (NULL)));#  [15/Sep/2024:11:22:13 -0400] \"GET /VERM/VERM_AJAX_functions.php?function=log_custom_report&SNROY=U5YFS HTTP/1.1\" 404 197 \"-\" \"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.6367.118 Safari/537.36\"".to_string();
        assert!(LogEntry::try_from(&line).is_ok());
        let le = LogEntry::try_from(&line).unwrap();
        assert_eq!(le.code, 404);
    }
}
