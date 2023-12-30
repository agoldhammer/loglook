use crate::log_entries::LogEntry;
use bson;
use chrono;
// use chrono::prelude::*;
use mongodb::bson::doc;
// use mongodb::error::Error;
#[allow(unused_imports)]
use futures::stream::{StreamExt, TryStreamExt};
use mongodb::{Collection, Cursor};

async fn query_date_range(
    coll: Collection<LogEntry>,
) -> Result<Cursor<LogEntry>, mongodb::error::Error> {
    let ct_time: chrono::DateTime<chrono::Utc> = "2023-12-29T00:00:00Z".parse().unwrap();
    let t: bson::DateTime = ct_time.into();
    let filter = doc! {"time": {"$gt": t}};
    coll.find(filter, None).await
}

pub async fn find_yesterday(coll: Collection<LogEntry>) {
    let mut _output_les: Vec<LogEntry> = vec![];
    let mut cursor = query_date_range(coll).await.unwrap(); //.map_err(anyhow::Error::msg);
    while let Some(maybe_le) = cursor.next().await {
        match maybe_le {
            Ok(maybe_le) => println!("ip time from lkup: {} {}", maybe_le.ip, maybe_le.time),
            Err(e) => eprintln!("{e}"),
        }
    }
}
