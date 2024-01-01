use console::style;
use mongodb::bson::doc;
use mongodb::options::IndexOptions;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines};
use std::path::PathBuf;
use std::vec::Vec;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use config_file::FromConfigFile;
use mongodb::{Client, Collection, IndexModel};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

pub mod geo;
pub mod lkup;
pub mod log_entries;
pub mod query;

use log_entries::LogEntry;

use crate::lkup::RevLookupData;

#[derive(Deserialize)]
#[allow(dead_code)]
struct Config {
    api_key: String,
    db_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HostData {
    ip: String,
    geodata: geo::Geodata,
    ptr_records: Vec<String>,
}

impl fmt::Display for HostData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{}: {}",
            style("IP").bold().red(),
            style(&self.ip).green()
        )
        .unwrap();

        write!(f, "{}", self.geodata).unwrap();
        self.ptr_records.iter().try_for_each(|record| {
            writeln!(f, "{}: {}", style("host").red(), style(record).green())
        })
    }
}
#[derive(Debug)]
pub struct Counts {
    pub n_logents: usize,
    pub n_unique_ips: usize,
    pub n_new_ips: usize,
    pub n_inserted_les: usize,
    pub n_skipped_les: usize,
}

fn read_config() -> Config {
    let path = shellexpand::tilde("~/.loglook/config.toml");
    let config = Config::from_config_file(path.as_ref()).unwrap();
    config
}

fn read_lines(path: &PathBuf) -> Result<io::Lines<BufReader<File>>, Box<dyn Error + 'static>> {
    let file = File::open(path)?;
    Ok(io::BufReader::new(file).lines())
}

fn logents_to_ips_set(logentries: &[LogEntry]) -> HashSet<String> {
    let mut ips = HashSet::new();
    for logentry in logentries {
        ips.insert(logentry.ip.clone());
    }
    ips
}

fn make_logentries(lines: Lines<BufReader<File>>) -> Vec<LogEntry> {
    let maybe_logentries: Vec<anyhow::Result<LogEntry>> = lines
        .map(|line| log_entries::LogEntry::try_from(&line.expect("log line should be readable")))
        .collect();
    // * deal with errors (poss bad lines in log) here by displaying on stderr
    let mut logentries: Vec<LogEntry> = Vec::new();
    for maybe_logentry in maybe_logentries.into_iter() {
        match maybe_logentry {
            Ok(logentry) => logentries.push(logentry),
            Err(e) => eprintln!("Log read error: {}", e),
        }
    }
    logentries
}

fn progress_bar_setup(n_pb_items: u64) -> (ProgressBar, ProgressBar) {
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");
    let pb_rdns = m.add(ProgressBar::new(n_pb_items));
    pb_rdns.set_style(sty.clone());
    pb_rdns.set_message("rdns");
    let pb_geo = m.add(ProgressBar::new(n_pb_items));
    pb_geo.set_style(sty.clone());
    pb_geo.set_message("geodata");
    (pb_rdns, pb_geo)
}

async fn setup_db(
    config: &Config,
) -> Result<(Collection<HostData>, Collection<LogEntry>), Box<dyn Error>> {
    let client = Client::with_uri_str(&config.db_uri).await?;
    // TODO: should take dbname from config
    let db = client.database("loglook");
    let host_data_coll: Collection<HostData> = db.collection("hostdata");
    let hd_options = IndexOptions::builder().unique(true).build();
    let hd_index_model = IndexModel::builder()
        .keys(doc! {"ip": 1})
        .options(hd_options)
        .build();
    #[allow(unused_variables)]
    let hd_index = host_data_coll.create_index(hd_index_model, None).await?;
    // * Indices on LogEntry collection
    // * Need several; first is compound on ip and time
    let logents_coll: Collection<LogEntry> = db.collection("logentries");
    let le_options = IndexOptions::builder().unique(true).build();
    // * need to include method in this index because ip+time is not enough to get uniqueness
    let le_index_model = IndexModel::builder()
        .keys(doc! {"ip": 1, "time": 1, "method": 1, "code": 1, "nbytes": 1})
        .options(le_options)
        .build();
    #[allow(unused_variables)]
    let le_index = logents_coll.create_index(le_index_model, None).await?;
    // * second is on time alone; non-unique
    let le_time_index_model = IndexModel::builder()
        .keys(doc! {"time": 1})
        .options(None)
        .build();
    logents_coll.create_index(le_time_index_model, None).await?;
    // println!("Indices {}, {}", hd_index.index_name, le_index.index_name);
    Ok((host_data_coll, logents_coll))
}

// * check if ip is already in HostData collection in db
async fn ip_in_hdcoll(
    ip: String,
    host_data_coll: Collection<HostData>,
) -> anyhow::Result<(String, bool)> {
    let query = doc! {"ip": ip.to_string()};
    let maybe_hd = host_data_coll.find_one(query, None).await?;
    let retval = match maybe_hd {
        Some(_) => (ip, true),
        None => (ip, false),
    };
    Ok(retval)
}

pub async fn run(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    /* Strategy: Parse loglines into LogEntries
    Do reverse dns lookup to generate RevLookupData, collect in map with ip as key
    Do geo lookup to generate Geodata, collect in map with ip as key
    Output to console
    Eventually, send to mongo db
     */
    let config = read_config();
    let mut counts = Counts {
        n_inserted_les: 0,
        n_logents: 0,
        n_new_ips: 0,
        n_skipped_les: 0,
        n_unique_ips: 0,
    };
    // * setup database
    let (host_data_coll, logents_coll) = setup_db(&config).await?;

    // ! for testing query
    query::find_yesterday3(logents_coll.clone()).await;
    // ! end test

    // * input stage
    let lines = read_lines(path)?;
    // * process each logline and collect parsed lines into Vec<LogEntry>
    let logentries = make_logentries(lines);
    counts.n_logents = logentries.len();
    // println!("Log lines: {le_count}");

    // * end of input stage, resulting in raw logentries

    // * from raw logentries extract set of unique ips and map from ips
    let ip_set = logents_to_ips_set(&logentries);

    // TODO spawn tasks to join all async calls
    let ips_all = ip_set.clone();
    let mut ips_join_set: JoinSet<(String, bool)> = JoinSet::new();
    for ip in ips_all {
        let hdc = host_data_coll.clone();
        ips_join_set.spawn(async move { ip_in_hdcoll(ip, hdc).await.unwrap() });
    }
    let mut ips_rdns_data_needed = HashSet::new();
    let mut ips_geodata_needed = HashSet::new();
    loop {
        let result = ips_join_set.join_next().await;
        match result {
            Some(result) => {
                let (ip, is_in) = result?;
                // println!("ip {} is in {}", ip, is_in);
                if !is_in {
                    ips_rdns_data_needed.insert(ip.clone());
                    ips_geodata_needed.insert(ip.clone());
                }
            }
            None => break,
        }
    }
    // * --------------
    counts.n_unique_ips = ip_set.len();
    counts.n_new_ips = ips_rdns_data_needed.len();
    let (pb_rdns, pb_geo) = progress_bar_setup(counts.n_unique_ips as u64);

    let mut ips_to_rdns_map: HashMap<String, RevLookupData> = HashMap::new();

    // * create channels to receive rev lkup results
    const CHAN_BUF_SIZE: usize = 256;
    let (tx_rdns, mut rx_rdns) = mpsc::channel(CHAN_BUF_SIZE);

    let mut join_set = JoinSet::new();
    for ip in ips_rdns_data_needed {
        let txa = tx_rdns.clone();
        join_set.spawn(async move { lkup::lkup_hostnames(&ip, txa).await });
    }

    let (tx_geo, mut rx_geo) = mpsc::channel(CHAN_BUF_SIZE);
    for ip in ips_geodata_needed {
        let txa2 = tx_geo.clone();
        let key = config.api_key.clone();
        join_set.spawn(async move { geo::geo_lkup(&ip, txa2, key).await });
    }

    // * output stuff
    drop(tx_rdns); // have to drop the original channel that has been cloned for each task
    drop(tx_geo);

    // ! only les associated with freshly looked up ips will be output here. Is that what is wanted?
    let mut ips_to_geodata_map: HashMap<String, geo::Geodata> = HashMap::new();
    while let Some(geo_lookup_data) = rx_geo.recv().await {
        pb_geo.inc(1);
        let ip = geo_lookup_data.ip.clone();
        ips_to_geodata_map.insert(ip.to_string(), geo_lookup_data);
    }

    while let Some(rev_lookup_data) = rx_rdns.recv().await {
        let ip = rev_lookup_data.ip_addr.clone();
        pb_rdns.inc(1);
        ips_to_rdns_map.insert(ip, rev_lookup_data);
    }

    pb_rdns.finish();
    pb_geo.finish();

    let mut ip_to_hostdata_map = HashMap::new();
    println!("\nOutput");
    for (ip, geodata) in ips_to_geodata_map {
        println!(
            "{}: {}",
            style("IP").bold().red(),
            style(&ip.clone()).green()
        );
        print!("{geodata}");
        let rdns = ips_to_rdns_map.get(&ip).unwrap();
        let hostdata = HostData {
            ip: ip.to_string(),
            geodata,
            ptr_records: rdns.ptr_records.clone(),
        };
        ip_to_hostdata_map.insert(ip.clone(), hostdata);
        println!("{rdns}\n");
        let les = logentries.iter().filter(|le| le.ip == ip);
        for le in les {
            println!("{le}");
        }
    }

    println!("Hostdata");
    for hd in ip_to_hostdata_map.values() {
        println!("{hd}");
    }

    let docs = ip_to_hostdata_map.values();
    if docs.len() > 0 {
        host_data_coll.insert_many(docs, None).await?;
    }
    // ! insertion of logentries is done synchronously with this logic. May want to change??
    for le in logentries {
        let result = logents_coll.insert_one(le, None).await;
        match result {
            Ok(_) => counts.n_inserted_les += 1,
            Err(_) => counts.n_skipped_les += 1,
        }
    }

    // * Display counts
    println!("Counts: {:?}", counts);
    // * end of output stuff

    while let Some(res) = join_set.join_next().await {
        res.expect("all async chans should finish");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // use super::*;
    use crate::read_config;
    // use mongodb::bson::{doc, to_document};
    // use tokio_test::assert_ok;

    // macro_rules! aw {
    //     ($e:expr) => {
    //         tokio_test::block_on($e)
    //     };
    // }

    #[test]
    fn config_read_rest() {
        let db_uri = read_config().db_uri;
        assert!(db_uri.contains("27017"));
    }

    // #[test]
    // fn item_in_db() {
    //     let config = read_config();
    //     let (hd_coll, _) = aw!(setup_db(&config)).unwrap();
    //     // 78.153.140.219
    //     let query = doc! {"geodata": {"ip": "78.153.140.219"}};
    //     let hd = aw!(hd_coll.find_one(query, None)).unwrap();
    //     // assert_eq!(Some(hd), { "ip" });
    // }
}
