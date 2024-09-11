// * handle geo lookups
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{fmt, sync::Arc};
use tokio::sync::mpsc;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Geodata {
    pub ip: String,
    pub country_name: String,
    pub state_prov: String,
    pub city: String,
    pub isp: String,
    pub organization: String,
}

impl Geodata {
    fn new(ip: &str) -> Geodata {
        Geodata {
            ip: String::from(ip),
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
        writeln!(
            f,
            "Loc: {}, {}, {}",
            self.city, self.state_prov, self.country_name
        )?;
        writeln!(f, "ISP: {}", self.isp)?;
        writeln!(f, "Org: {}", self.organization)
    }
}

// * send error message encapsulated in a Geodata struct
async fn send_error(tx: mpsc::Sender<Geodata>, ip: &str, msg: &str) {
    let mut geod = Geodata::new(ip);
    // geod.ip = format!("{}", ip).to_string();
    geod.city = format!("Error in geodata lookup: {}", msg).to_string();
    tx.send(geod).await.expect("shd send geod error");
}

pub async fn geo_lkup(ip: &str, tx: mpsc::Sender<Geodata>, api_key: Arc<String>) {
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
                send_error(tx, ip, &msg).await;
            }
        };
    } else {
        let msg = format!("error acquiring geodata for IP {:?}", ip);
        send_error(tx, ip, &msg).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_config;
    // use tokio_test::assert_err;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn geo_lkup_bad_ip() {
        let conf = read_config().unwrap();
        let api_key = conf.api_key;
        let key = Arc::new(api_key);
        let (tx, _rx) = mpsc::channel(32);
        let ip = "192.168.0.116";
        assert_eq!(aw!(geo_lkup(ip, tx, key)), ());
    }
}
