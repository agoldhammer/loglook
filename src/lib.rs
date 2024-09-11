use anyhow::{anyhow, bail};
use console::style;
use futures::StreamExt;
use mongodb::bson::doc;
use mongodb::options::IndexOptions;
use query::DateRange;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines};
use std::path::PathBuf;
use std::sync::Arc;
use std::vec::Vec;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use config_file::FromConfigFile;
use mongodb::{Client, Collection, IndexModel};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

pub mod geo;
pub mod lkup;
pub mod log_entries;
pub mod query;

use log_entries::LogEntry;

use crate::lkup::RevLookupData;

type Logdate = chrono::DateTime<chrono::Utc>;
type LogEntryColl = Collection<LogEntry>;
type HostDataColl = Collection<HostData>;

#[derive(Deserialize)]
pub struct Config {
    pub api_key: String,
    pub db_uri: String,
    pub db_name: String, // canonical name is loglook for prod, test_loglook for dev
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostData {
    pub ip: String,
    pub geodata: geo::Geodata,
    pub ptr_records: Vec<String>,
}

impl fmt::Display for HostData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{}: {}",
            style("IP").bold().red(),
            style(&self.ip).green()
        )?;

        write!(f, "{}", self.geodata)?;
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

pub fn read_config() -> anyhow::Result<Config> {
    let path = shellexpand::tilde("~/.loglook/config.toml");
    println!("Reading config from: {}", path.to_string());
    let config = Config::from_config_file(path.as_ref())?;
    Ok(config)
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
) -> anyhow::Result<(mongodb::Database, HostDataColl, LogEntryColl)> {
    let client = Client::with_uri_str(&config.db_uri).await?;
    // TODO: should take dbname from config
    let db = client.database(&config.db_name);
    let host_data_coll: HostDataColl = db.collection("hostdata");
    let hd_options = IndexOptions::builder().unique(true).build();
    let hd_index_model = IndexModel::builder()
        .keys(doc! {"ip": 1})
        .options(hd_options)
        .build();
    #[allow(unused_variables)]
    let hd_index = host_data_coll.create_index(hd_index_model, None).await?;
    let hd_country_index_model = IndexModel::builder()
        .keys(doc! {"geodata.country": 1})
        .options(None)
        .build();
    host_data_coll
        .create_index(hd_country_index_model, None)
        .await?;
    // * Indices on LogEntry collection
    // * Need several; first is compound on ip and time
    let logents_coll: LogEntryColl = db.collection("logentries");
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
    Ok((db, host_data_coll, logents_coll))
}

// * check if ip is already in HostData collection in db
async fn ip_in_hdcoll(ip: String, host_data_coll: HostDataColl) -> anyhow::Result<(String, bool)> {
    let query = doc! {"ip": ip.to_string()};
    let maybe_hd = host_data_coll.find_one(query, None).await?;
    let retval = match maybe_hd {
        Some(_) => (ip, true),
        None => (ip, false),
    };
    Ok(retval)
}

pub async fn read(daemon: &bool, path: &PathBuf, config: &Config) -> anyhow::Result<()> {
    /* Strategy: Parse loglines into LogEntries
    Do reverse dns lookup to generate RevLookupData, collect in map with ip as key
    Do geo lookup to generate Geodata, collect in map with ip as key
    Output to console
    Eventually, send to mongo db
     */
    let mut counts = Counts {
        n_inserted_les: 0,
        n_logents: 0,
        n_new_ips: 0,
        n_skipped_les: 0,
        n_unique_ips: 0,
    };
    // * setup database
    let (_, host_data_coll, logents_coll) = setup_db(config).await?;

    // * input stage
    let file = read_lines(path);
    let lines = match file {
        Ok(file) => file,
        Err(e) => bail!("Error reading log file: {e}"),
    };
    // * process each logline and collect parsed lines into Vec<LogEntry>
    let logentries = make_logentries(lines);
    counts.n_logents = logentries.len();
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
    while let Some(res) = ips_join_set.join_next().await {
        let (ip, is_in) = res?;
        if !is_in {
            ips_rdns_data_needed.insert(ip.clone());
            ips_geodata_needed.insert(ip.clone());
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
    let api_key = Arc::new(config.api_key.clone());
    for ip in ips_geodata_needed {
        let txa2 = tx_geo.clone();
        let key = api_key.clone();
        join_set.spawn(async move { geo::geo_lkup(&ip, txa2, key).await });
    }

    // * output stuff
    drop(tx_rdns); // have to drop the original channel that has been cloned for each task
    drop(tx_geo);

    //  only les associated with freshly looked up ips will be output here. Is that what is wanted?
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
    if !daemon {
        println!("\nOutput");
    }
    for (ip, geodata) in ips_to_geodata_map {
        if !daemon {
            println!(
                "{}: {}",
                style("IP").bold().red(),
                style(&ip.clone()).green()
            );
            print!("{geodata}");
        }
        let rdns = ips_to_rdns_map.get(&ip).unwrap();
        let hostdata = HostData {
            ip: ip.to_string(),
            geodata,
            ptr_records: rdns.ptr_records.clone(),
        };
        ip_to_hostdata_map.insert(ip.clone(), hostdata);
        if !daemon {
            println!("{rdns}\n");
            let les = logentries.iter().filter(|le| le.ip == ip);
            for le in les {
                println!("{le}");
            }
        }
    }

    if !daemon {
        println!("Hostdata");
        for hd in ip_to_hostdata_map.values() {
            println!("{hd}");
        }
    }

    let docs = ip_to_hostdata_map.values();
    if docs.len() > 0 {
        host_data_coll.insert_many(docs, None).await?;
    }
    //  insertion of logentries is done synchronously with this logic. May want to change??
    for le in logentries {
        let result = logents_coll.insert_one(le, None).await;
        match result {
            Ok(_) => counts.n_inserted_les += 1,
            Err(_) => counts.n_skipped_les += 1,
        }
    }

    // * Display counts
    println!("Read result: {:?}", counts);
    // * end of output stuff

    while let Some(res) = join_set.join_next().await {
        res.expect("all async chans should finish");
    }

    Ok(())
}

// given an ip, lookup hostdata
pub async fn get_hostdata(ip: &str, hd_coll: &HostDataColl) -> anyhow::Result<HostData> {
    let maybe_hd = hd_coll.find_one(doc! {"ip": ip}, None).await?;
    match maybe_hd {
        Some(hd) => Ok(hd),
        None => Err(anyhow!("ip not found")),
    }
}

// output a vector of ips
async fn output_ips(
    suppress_logentry_output: bool,
    hostdata_coll: &Collection<HostData>,
    current_logentries_coll: &Collection<LogEntry>,
    ips: &mut [String],
) -> anyhow::Result<()> {
    ips.sort();
    for ip in ips.iter() {
        let hd = get_hostdata(ip, hostdata_coll).await?;
        println!("{}", hd);
        if !suppress_logentry_output {
            let filter = doc! {"ip": ip};
            let mut curs = current_logentries_coll.find(filter, None).await?;

            while let Some(le) = curs.next().await {
                let lex = le?;
                println!("{}", lex);
            }
        }
    }
    Ok(())
}

async fn output_all_current_ips(
    suppress_logentry_output: bool,
    hostdata_coll: &Collection<HostData>,
    logents_coll: &Collection<LogEntry>,
    current_logentries_coll: &Collection<LogEntry>,
    date_range: &DateRange,
) -> anyhow::Result<()> {
    let mut ips_in_daterange = query::find_ips_in_daterange(logents_coll, date_range).await?;
    output_ips(
        suppress_logentry_output,
        hostdata_coll,
        current_logentries_coll,
        &mut ips_in_daterange,
    )
    .await?;
    Ok(())
}

pub async fn search(
    nologs: &Option<bool>,
    start: &str,
    end: &str,
    ip: &Option<String>,
    country: &Option<Vec<String>>,
    org: &Option<String>,
    config: &Config,
) -> anyhow::Result<()> {
    let suppress_logentry_output: bool = match *nologs {
        Some(true) => true,
        Some(false) => false,
        None => false,
    };

    let date_range = query::time_str_to_daterange(start, end)?;
    let (loglook_db, hostdata_coll, logents_coll) = setup_db(config).await?;
    // * set up the temporary collection of logentries using the current daterange, name = "current_logentries"
    query::make_current_le_coll(&date_range, &logents_coll).await?;
    let current_logentries_coll: mongodb::Collection<LogEntry> =
        loglook_db.collection("current_logentries");
    if country.is_some() || ip.is_some() || org.is_some() {
        match (ip, country, org) {
            (None, Some(country), None) => {
                // * get vec of all CountryWithIps in current daterange, then pass only those
                //  * corresponding to an accepted country (as specified on command line)
                // * or output
                let country_with_ips_vec =
                    query::get_current_ips_by_country(&current_logentries_coll).await?;
                for mut country_with_ips in country_with_ips_vec {
                    // * The country cli arg yields a vector of countries to accept
                    // * If no countries are specified after --country flag, all are accepted
                    let current_country = &country_with_ips.country;
                    let accepted_countries = country;
                    if accepted_countries.contains(current_country) || accepted_countries.is_empty()
                    {
                        println!(
                            "{}: {}\n----------",
                            style("Country").red(),
                            style(country_with_ips.country).yellow()
                        );
                        output_ips(
                            suppress_logentry_output,
                            &hostdata_coll,
                            &current_logentries_coll,
                            &mut country_with_ips.ips,
                        )
                        .await?;
                    };
                }
            }
            // * search for ip with regex
            (Some(ip), None, None) => {
                let mut ips = query::find_ips_matching_regex(&current_logentries_coll, ip).await?;
                output_ips(
                    suppress_logentry_output,
                    &hostdata_coll,
                    &current_logentries_coll,
                    &mut ips,
                )
                .await?;
            }
            // * search for org with regex
            (None, None, Some(org)) => {
                let orgs_with_ips = query::get_current_ips_by_org(&current_logentries_coll).await?;
                let re = Regex::new(org)?;
                for org_with_ips in orgs_with_ips {
                    if re.is_match(&org_with_ips.org) {
                        println!(
                            "{}: {}\n----------",
                            style("Organization").red(),
                            style(&org_with_ips.org).yellow()
                        );
                        let mut ips = org_with_ips.ips;
                        output_ips(
                            suppress_logentry_output,
                            &hostdata_coll,
                            &current_logentries_coll,
                            &mut ips,
                        )
                        .await?;
                    }
                }
            }
            _ => (),
        };
    } else {
        output_all_current_ips(
            suppress_logentry_output,
            &hostdata_coll,
            &logents_coll,
            &current_logentries_coll,
            &date_range,
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::assert_ok;
    use tokio_test::block_on;

    macro_rules! aw {
        ($e:expr) => {
            block_on($e)
        };
    }

    #[test]
    fn config_read_test() {
        let config = read_config().unwrap();
        assert!(config.db_uri.contains("27017"));
        assert!(config.db_name.contains("loglook"));
    }

    #[test]
    fn test_search() {
        let config = read_config().unwrap();
        let nologs = None as Option<bool>;
        let start = "2023-11-25T00:00:00Z";
        let end = "2023-11-26T00:00:00Z";
        let void_arg = None as Option<String>;
        let void_vec = None as Option<Vec<String>>;
        let res = aw!(search(
            &nologs, start, end, &void_arg, &void_vec, &void_arg, &config
        ));
        tokio_test::assert_ok!(res);
    }
}
