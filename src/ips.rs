use std::{collections::HashSet, net::IpAddr};
// use dns_lookup::lookup_addr;

pub(crate) fn printips(ips: &HashSet<IpAddr>) {
    let n_ips = ips.len();

    for ip in ips {
        println!("{ip}");
    }
    println!("\n# unique ips: {}", n_ips);
}

//// fn get_hostname(ip: &IpAddr) {
//     match lookup_addr(ip) {
//         Ok(hostname) => {println!("host: {}", hostname)}
//         Err(e) => {println!("err looking up {}", e)}
//     }
// }
