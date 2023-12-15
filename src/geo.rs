// * handle geo lookups
use config_file::FromConfigFile;
use reqwest;
use serde::Deserialize;
use serde_json;
use shellexpand;
// use std::error::Error;
use std::{fmt, net::IpAddr};
use tokio::sync::mpsc;

// use error_chain::error_chain;

// error_chain! {
//     foreign_links {
//         Io(std::io::Error);
//         HttpRequest(reqwest::Error);
//     }
// }

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

// TODO send results out over channel
// ! see also https://users.rust-lang.org/t/propagating-errors-from-tokio-tasks/41723/4
pub async fn geo_lkup(ip: IpAddr, _tx: mpsc::Sender<Geodata>) -> Result<(), Option<String>> {
    let api_key = read_config();
    let uri = format!("https://api.ipgeolocation.io/ipgeo?apiKey={api_key}&ip={ip}");
    let text = reqwest::get(uri).await.expect("url error").text().await;
    let geodata: Geodata = serde_json::from_str(&text.expect("api error")).expect("json error");
    println!("Geodata\n {}", geodata);
    Ok(())
}
