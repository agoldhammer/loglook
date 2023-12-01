
use std::{net::IpAddr, collections::HashSet};


pub(crate) fn printip (ip: IpAddr) {
    println!("{}", ip);
}

pub(crate) fn printips(ips:  HashSet<IpAddr>) {
    let n_ips = ips.len();

    for ip in ips {
        printip(ip);
    }
    println!("\n# unique ips: {}",  n_ips);
}