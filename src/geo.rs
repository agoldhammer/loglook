// * handle geo lookups
use config_file::FromConfigFile;
use serde::Deserialize;
use shellexpand;

#[derive(Deserialize)]
struct Config {
    api_key: String,
}

fn read_config() -> String {
    let path = shellexpand::tilde("~/Prog/loglook/src/config.toml");
    let config = Config::from_config_file(path.as_ref()).unwrap();
    config.api_key
}

pub async fn geo_lkup() {
    let api_key = read_config();
    println!("key: {api_key}");
}
