// * handle geo lookups
use config_file::FromConfigFile;
use reqwest;
use serde::Deserialize;
use serde_json;
use shellexpand;
// use std::error::Error;
use std::{fmt, net::IpAddr};
use tokio::sync::mpsc;

#[derive(Deserialize)]
struct Config {
    api_key: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Geodata {
    pub ip: IpAddr,
    country_name: String,
    state_prov: String,
    city: String,
    isp: String,
    organization: String,
}

impl Geodata {
    fn new(ip: &IpAddr) -> Geodata {
        Geodata {
            ip: *ip,
            country_name: "".to_string(),
            state_prov: "".to_string(),
            city: "".to_string(),
            isp: "".to_string(),
            organization: "".to_string(),
        }
    }
}

impl fmt::Display for Geodata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // write!(f, "IP: {}\n", self.ip).unwrap();
        writeln!(
            f,
            "Loc: {}, {}, {}",
            self.city, self.state_prov, self.country_name
        )
        .unwrap();
        write!(f, "ISP: {}, Org: {}", self.isp, self.organization)
    }
}

pub fn read_config() -> String {
    let path = shellexpand::tilde("~/.loglook/config.toml");
    let config = Config::from_config_file(path.as_ref()).unwrap();
    config.api_key
}

// send error message encapsulated in a Geodata struct
async fn send_error(tx: mpsc::Sender<Geodata>, ip: &IpAddr, msg: &str) {
    let mut geod = Geodata::new(ip);
    // geod.ip = format!("{}", ip).to_string();
    geod.city = format!("Error in geodata lookup: {}", msg).to_string();
    tx.send(geod).await.expect("shd send geod error");
}

pub async fn geo_lkup(ip: IpAddr, tx: mpsc::Sender<Geodata>, api_key: String) {
    let uri = format!("https://api.ipgeolocation.io/ipgeo?apiKey={api_key}&ip={ip}");
    let res = reqwest::get(uri).await.unwrap();
    if res.status() == 200 {
        let text = res.text().await.unwrap();
        match serde_json::from_str(&text) as Result<Geodata, serde_json::Error> {
            Ok(geodata) => {
                tx.send(geodata).await.expect("geodata send shd work");
            }
            Err(e) => {
                let msg = format!("error decoding json {}", e);
                send_error(tx, &ip, &msg).await;
            }
        };
    } else {
        let msg = format!("error acquiring geodata for IP {:?}", ip);
        send_error(tx, &ip, &msg).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use tokio_test::assert_err;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn geo_lkup_bad_ip() {
        let api_key = read_config();
        let (tx, _rx) = mpsc::channel(32);
        let ip: IpAddr = "192.168.0.116".parse().unwrap();
        assert_eq!(aw!(geo_lkup(ip, tx, api_key)), ());
    }
}
