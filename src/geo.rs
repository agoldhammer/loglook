// * handle geo lookups
use config_file::FromConfigFile;
use reqwest;
use serde::Deserialize;
use serde_json;
use shellexpand;
use std::{fmt, net::IpAddr};
use tokio::sync::mpsc;

#[derive(Deserialize)]
struct Config {
    api_key: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Geodata {
    ip: String,
    country_name: String,
    state_prov: String,
    city: String,
    isp: String,
    organization: String,
}

impl fmt::Display for Geodata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IP: {}\n", self.ip).unwrap();
        write!(
            f,
            "Loc: {}, {}, {}\n",
            self.city, self.state_prov, self.country_name
        )
        .unwrap();
        write!(f, "ISP: {}, Org: {}\n", self.isp, self.organization)
    }
}

fn read_config() -> String {
    let path = shellexpand::tilde("~/Prog/loglook/src/config.toml");
    let config = Config::from_config_file(path.as_ref()).unwrap();
    config.api_key
}

pub async fn geo_lkup(ip: IpAddr, _tx: mpsc::Sender<Geodata>) {
    let api_key = read_config();
    let uri = format!("https://api.ipgeolocation.io/ipgeo?apiKey={api_key}&ip={ip}");
    let maybe_body = reqwest::get(uri).await;
    let maybe_text = maybe_body.unwrap().text().await;
    let text = maybe_text.unwrap(); //.text().await;
    let geodata: Geodata = serde_json::from_str(&text).unwrap();
    // match geodata {
    //     Ok(geodata) => geodata,
    //     Err(e) => {
    //         eprintln!("geo lkup failed: err {}, ip {}", e, ip)
    //     }
    // }
    println!("Geodata\n {}", geodata);
}
