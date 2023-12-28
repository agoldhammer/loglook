use console::style;
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use hickory_resolver::TokioAsyncResolver;

use tokio::sync::mpsc;
use tokio::time::timeout;

#[derive(Debug)]
pub struct RevLookupData {
    pub ip_addr: String,
    pub ptr_records: Vec<String>,
}

impl RevLookupData {
    fn new(ip_addr: String) -> RevLookupData {
        RevLookupData {
            ip_addr,
            ptr_records: Vec::new(),
        }
    }
}

impl fmt::Display for RevLookupData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // write!(f, "ip: {}: ", self.ip_addr).unwrap();
        self.ptr_records
            .iter()
            .try_for_each(|record| write!(f, "{}: {}", style("host").red(), style(record).green()))
    }
}

// * Do reverse lookup on ip_str, send result out on channel tx
pub async fn lkup_hostnames(ip: &str, tx: mpsc::Sender<RevLookupData>) {
    const TIMEOUT_MS: u64 = 1000;
    let resolver = TokioAsyncResolver::tokio_from_system_conf().unwrap();

    let reverse_lookup = resolver.reverse_lookup(IpAddr::from_str(ip).unwrap());
    let timeout_duration = Duration::from_millis(TIMEOUT_MS);
    let lookup_result = timeout(timeout_duration, reverse_lookup).await;
    let mut rev_lookup_data = RevLookupData::new(ip.to_string());
    match lookup_result {
        Ok(Ok(lookup_result)) => {
            // * successful lookup
            rev_lookup_data.ptr_records = lookup_result
                .iter()
                .map(|record| format!("{}", record))
                .collect();
        }
        Ok(Err(_)) => rev_lookup_data.ptr_records.push("unknown".to_string()), //no PTR records found,
        Err(_) => rev_lookup_data.ptr_records.push("timed out".to_string()),   // lookup timed out
    };
    tx.send(rev_lookup_data).await.expect("should just work");
}
