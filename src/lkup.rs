use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

use hickory_resolver::TokioAsyncResolver;

use tokio::sync::mpsc;
// use tokio::task::JoinSet;
use tokio::time::timeout;

#[derive(Debug)]
pub struct RevLookupData {
    pub ip_addr: IpAddr,
    pub ptr_records: Vec<String>,
}

impl RevLookupData {
    fn new(ip_addr: IpAddr) -> RevLookupData {
        RevLookupData {
            ip_addr,
            ptr_records: Vec::new(),
        }
    }
}

impl fmt::Display for RevLookupData {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ip: {}: ", self.ip_addr).unwrap();
        self.ptr_records
            .iter()
            .try_for_each(|record| write!(f, "host: {}", record))
    }
}

// * Do reverse lookup on ip_str, send result out on channel tx
pub async fn lkup_hostnames(ip: IpAddr, tx: mpsc::Sender<RevLookupData>) {
    // async fn get_name(ip_str: &String)  . {
    const TIMEOUT_MS: u64 = 2500;
    // let ip_addr: IpAddr = ip_str.parse().unwrap();
    let resolver = TokioAsyncResolver::tokio_from_system_conf().unwrap();

    let reverse_lookup = resolver.reverse_lookup(ip);
    let timeout_duration = Duration::from_millis(TIMEOUT_MS);
    let lookup_result = timeout(timeout_duration, reverse_lookup).await;
    let mut rev_lookup_data = RevLookupData::new(ip);
    match lookup_result {
        Ok(Ok(lookup_result)) => {
            //successful lookup
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
