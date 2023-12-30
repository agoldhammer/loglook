use crate::log_entries::LogEntry;
use bson;
use bson::from_document;
use bson::Document;
use chrono;
use serde::Deserialize;
// use chrono::prelude::*;
use mongodb::bson::doc;
// use mongodb::error::Error;
#[allow(unused_imports)]
use futures::stream::{StreamExt, TryStreamExt};
use mongodb::{Collection, Cursor};

#[derive(Deserialize)]
#[allow(dead_code)]
struct LeGroup {
    _id: String,
    les: Vec<LogEntry>,
}

async fn query_date_range(
    coll: Collection<LogEntry>,
) -> Result<Cursor<LogEntry>, mongodb::error::Error> {
    let ct_start: chrono::DateTime<chrono::Utc> = "2023-12-30T16:00:00Z".parse().unwrap();
    let ct_end: chrono::DateTime<chrono::Utc> = "2023-12-30T20:00:00Z".parse().unwrap();
    let s: bson::DateTime = ct_start.into();
    let e: bson::DateTime = ct_end.into();
    let filter = doc! {"time": {"$gte": s, "$lt": e}};
    // let time_filter = doc! {"$match": {"time": {"$gte": s, "$lt": e}}};
    // let sort_by_ip = doc! {"$sort": {"ip": 1}};
    // let pipeline = vec![time_filter, sort_by_ip];
    coll.find(filter, None).await
    // coll.aggregate(pipeline, None).await
}

async fn query_date_range_pl(
    coll: Collection<LogEntry>,
) -> Result<Cursor<Document>, mongodb::error::Error> {
    let ct_start: chrono::DateTime<chrono::Utc> = "2023-12-30T16:00:00Z".parse().unwrap();
    let ct_end: chrono::DateTime<chrono::Utc> = "2023-12-30T20:00:00Z".parse().unwrap();
    let s: bson::DateTime = ct_start.into();
    let e: bson::DateTime = ct_end.into();
    // let filter = doc! {"time": {"$gte": s, "$lt": e}};
    let time_filter = doc! {"$match": {"time": {"$gte": s, "$lt": e}}};
    let sort_by_ip = doc! {"$sort": {"ip": 1}};
    let pipeline = vec![time_filter, sort_by_ip];
    // coll.find(filter, None).await
    coll.aggregate(pipeline, None).await
}

// async fn group_les_by_ip_and_date(
//     coll: Collection<LogEntry>,
// ) -> Result<Cursor<Document>, mongodb::error::Error> {
//     let ct_start: chrono::DateTime<chrono::Utc> = "2023-12-30T16:00:00Z".parse().unwrap();
//     let ct_end: chrono::DateTime<chrono::Utc> = "2023-12-30T20:00:00Z".parse().unwrap();
//     let s: bson::DateTime = ct_start.into();
//     let e: bson::DateTime = ct_end.into();
//     // let filter = doc! {"time": {"$gte": s, "$lt": e}};
//     let time_filter = doc! {"$match": {"time": {"$gte": s, "$lt": e}}};
//     let sort_by_ip = doc! {"$sort": {"ip": 1}};
//     let grouper = doc! {"$group": {"_id": "ip", "les": {"$push": ???}}};
//     let pipeline = vec![time_filter, sort_by_ip, grouper];
//     // coll.find(filter, None).await
//     coll.aggregate(pipeline, None).await
// }

pub async fn find_yesterday(coll: Collection<LogEntry>) {
    let mut _output_les: Vec<LogEntry> = vec![];
    let mut cursor = query_date_range(coll).await.unwrap(); //.map_err(anyhow::Error::msg);
    while let Some(maybe_le) = cursor.next().await {
        match maybe_le {
            Ok(maybe_le) => println!("yester: {}\n{}", maybe_le.ip, maybe_le),
            Err(e) => eprintln!("{e}"),
        }
    }
}

pub async fn find_yesterday2(coll: Collection<LogEntry>) {
    let mut _output_les: Vec<LogEntry> = vec![];
    let mut cursor = query_date_range_pl(coll).await.unwrap(); //.map_err(anyhow::Error::msg);
    while let Some(maybe_le) = cursor.next().await {
        match maybe_le {
            Ok(maybe_le) => {
                let le: LogEntry = from_document(maybe_le).unwrap();
                println!("yester: {}\n{}", le.ip, le);
            }

            Err(e) => eprintln!("{e}"),
        }
    }
}

// pub async fn find_yesterday3(coll: Collection<LogEntry>) {
//     let mut _output_les: Vec<LogEntry> = vec![];
//     let mut cursor = group_les_by_ip_and_date(coll).await.unwrap(); //.map_err(anyhow::Error::msg);
//     while let Some(maybe_le_group) = cursor.next().await {
//         match maybe_le_group {
//             Ok(maybe_le_group) => {
//                 let grp: LeGroup = from_document(maybe_le_group).unwrap();
//                 println!("yester: {} count {}", grp._id, grp.les.len());
//             }

//             Err(e) => eprintln!("{e}"),
//         }
//     }
// }
