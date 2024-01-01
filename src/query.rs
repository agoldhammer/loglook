use crate::log_entries::LogEntry;
use bson;
// use bson::from_document;
use bson::Bson;
use bson::Document;
use chrono;
// use serde::Deserialize;
// use chrono::prelude::*;
use mongodb::bson::doc;
// use mongodb::error::Error;
#[allow(unused_imports)]
use futures::stream::{StreamExt, TryStreamExt};
use mongodb::{Collection, Cursor};
type IpsInDaterange = Vec<String>;

// #[derive(Deserialize)]
// #[allow(dead_code)]
// struct LeGroup {
//     _id: String,
//     les: Vec<LogEntry>,
// }

// async fn query_date_range(
//     coll: Collection<LogEntry>,
// ) -> Result<Cursor<LogEntry>, mongodb::error::Error> {
//     let ct_start: chrono::DateTime<chrono::Utc> = "2023-12-30T16:00:00Z".parse().unwrap();
//     let ct_end: chrono::DateTime<chrono::Utc> = "2023-12-30T20:00:00Z".parse().unwrap();
//     let s: bson::DateTime = ct_start.into();
//     let e: bson::DateTime = ct_end.into();
//     let filter = doc! {"time": {"$gte": s, "$lt": e}};
//     // let time_filter = doc! {"$match": {"time": {"$gte": s, "$lt": e}}};
//     // let sort_by_ip = doc! {"$sort": {"ip": 1}};
//     // let pipeline = vec![time_filter, sort_by_ip];
//     coll.find(filter, None).await
//     // coll.aggregate(pipeline, None).await
// }

// async fn query_date_range_pl(
//     coll: Collection<LogEntry>,
// ) -> Result<Cursor<Document>, mongodb::error::Error> {
//     let ct_start: chrono::DateTime<chrono::Utc> = "2023-12-30T16:00:00Z".parse().unwrap();
//     let ct_end: chrono::DateTime<chrono::Utc> = "2023-12-30T20:00:00Z".parse().unwrap();
//     let s: bson::DateTime = ct_start.into();
//     let e: bson::DateTime = ct_end.into();
//     // let filter = doc! {"time": {"$gte": s, "$lt": e}};
//     let time_filter = doc! {"$match": {"time": {"$gte": s, "$lt": e}}};
//     let sort_by_ip = doc! {"$sort": {"ip": 1}};
//     let pipeline = vec![time_filter, sort_by_ip];
//     // coll.find(filter, None).await
//     coll.aggregate(pipeline, None).await
// }

async fn get_unique_ips_in_daterange(
    coll: Collection<LogEntry>,
) -> Result<Cursor<Document>, mongodb::error::Error> {
    let ct_start: chrono::DateTime<chrono::Utc> = "2023-12-30T16:00:00Z".parse().unwrap();
    let ct_end: chrono::DateTime<chrono::Utc> = "2023-12-30T20:00:00Z".parse().unwrap();
    let s: bson::DateTime = ct_start.into();
    let e: bson::DateTime = ct_end.into();
    // let filter = doc! {"time": {"$gte": s, "$lt": e}};
    let time_filter = doc! {"$match": {"time": {"$gte": s, "$lt": e}}};
    // let sort_by_ip = doc! {"$sort": {"ip": 1}};
    let grouper = doc! {"$group": {"_id": "$ip"}};
    let pipeline = vec![time_filter, grouper];
    // coll.find(filter, None).await
    coll.aggregate(pipeline, None).await
}

// pub async fn find_yesterday(coll: Collection<LogEntry>) {
// let mut _output_les: Vec<LogEntry> = vec![];
// let mut cursor = query_date_range(coll).await.unwrap(); //.map_err(anyhow::Error::msg);
// while let Some(maybe_le) = cursor.next().await {
//     match maybe_le {
//         Ok(maybe_le) => println!("yester: {}\n{}", maybe_le.ip, maybe_le),
//         Err(e) => eprintln!("{e}"),
//     }
// }
// }

// pub async fn find_yesterday2(coll: Collection<LogEntry>) {
//     let mut _output_les: Vec<LogEntry> = vec![];
//     let mut cursor = query_date_range_pl(coll).await.unwrap(); //.map_err(anyhow::Error::msg);
//     while let Some(maybe_le) = cursor.next().await {
//         match maybe_le {
//             Ok(maybe_le) => {
//                 let le: LogEntry = from_document(maybe_le).unwrap();
//                 println!("yester: {}\n{}", le.ip, le);
//             }

//             Err(e) => eprintln!("{e}"),
//         }
//     }
// }

pub async fn find_ips_in_daterange(coll: Collection<LogEntry>) -> IpsInDaterange {
    let mut cursor = get_unique_ips_in_daterange(coll).await.unwrap();
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

pub async fn find_yesterday3(coll: Collection<LogEntry>) {
    let ips_in_dr = find_ips_in_daterange(coll).await;
    println! {"ips in dr: {:?}", ips_in_dr};
}
