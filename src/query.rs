use super::Logdate;
use crate::log_entries::LogEntry;
use bson;
use bson::Bson;
use bson::Document;
use mongodb::bson::doc;
use std::error::Error;
// use mongodb::error::Error;
// use anyhow::anyhow;
#[allow(unused_imports)]
use futures::stream::StreamExt;
use mongodb::{Collection, Cursor};

type IpsInDaterange = Vec<String>;

async fn get_unique_ips_in_daterange(
    coll: Collection<LogEntry>,
    start_utc: Logdate,
    end_utc: Logdate,
) -> anyhow::Result<Cursor<Document>, mongodb::error::Error> {
    // let ct_start: chrono::DateTime<chrono::Utc> = "2023-12-30T16:00:00Z".parse().unwrap();
    // let ct_end: chrono::DateTime<chrono::Utc> = "2023-12-30T20:00:00Z".parse().unwrap();
    let s: bson::DateTime = start_utc.into();
    let e: bson::DateTime = end_utc.into();
    // let filter = doc! {"time": {"$gte": s, "$lt": e}};
    let time_filter = doc! {"$match": {"time": {"$gte": s, "$lt": e}}};
    // let sort_by_ip = doc! {"$sort": {"ip": 1}};
    let grouper = doc! {"$group": {"_id": "$ip"}};
    let pipeline = vec![time_filter, grouper];
    // coll.find(filter, None).await
    coll.aggregate(pipeline, None).await
}

async fn find_ips_in_daterange(
    coll: Collection<LogEntry>,
    start_utc: Logdate,
    end_utc: Logdate,
) -> IpsInDaterange {
    let mut cursor = get_unique_ips_in_daterange(coll, start_utc, end_utc)
        .await
        .unwrap();
    let mut ips_in_daterange: IpsInDaterange = vec![];
    while let Some(maybe_ipdoc) = cursor.next().await {
        // eprintln!("there are some");
        match maybe_ipdoc {
            Ok(maybe_ipdoc) => {
                let ipdoc: Document = maybe_ipdoc;
                // eprintln!("ipdoc {ipdoc}");
                let id = ipdoc.get("_id").unwrap();
                match id {
                    Bson::String(id) => ips_in_daterange.push(id.clone()),
                    _ => panic!("expected id to be a string"),
                }
            }

            Err(e) => eprintln!("{e}"),
        }
    }
    ips_in_daterange
}

// TODO: Make fn above public, fix up error handling so can be called from lib.rs
// https://antoinerr.github.io/blog-website/2023/01/28/rust-anyhow.html
pub async fn find_yesterday3(
    coll: Collection<LogEntry>,
    start_utc: Logdate,
    end_utc: Logdate,
) -> Result<(), Box<dyn Error>> {
    let ips_in_dr = find_ips_in_daterange(coll, start_utc, end_utc).await;
    println! {"ips in dr: {:?}", ips_in_dr};
    Ok(())
}
