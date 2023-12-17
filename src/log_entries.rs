// LogEntry holds info derived from one line of log file
use console::style;
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
            style(&self.hostname).magenta()
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
