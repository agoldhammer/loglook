// * handle geo lookups
use config_file::FromConfigFile;
use reqwest;
use serde::Deserialize;
use serde_json;
use shellexpand;
use std::error::Error;

#[derive(Deserialize)]
struct Config {
    api_key: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Geodata {
    country_name: String,
    state_prov: String,
    city: String,
    isp: String,
    organization: String,
}

fn read_config() -> String {
    let path = shellexpand::tilde("~/Prog/loglook/src/config.toml");
    let config = Config::from_config_file(path.as_ref()).unwrap();
    config.api_key
}

pub async fn geo_lkup() -> Result<(), Box<dyn Error>> {
    let api_key = read_config();
    let uri = format!("https://api.ipgeolocation.io/ipgeo?apiKey={api_key}&ip=8.8.8.8");
    let body = reqwest::get(uri).await?.text().await?;
    let geodata: Geodata = serde_json::from_str(&body).unwrap();
    dbg!(geodata);
    Ok(())
}
