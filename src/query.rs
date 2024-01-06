use super::HostData;
use super::Logdate;
use crate::log_entries::LogEntry;
use bson;
use bson::Document;
use bson::{Bson, DateTime};
use futures::stream::StreamExt;
use mongodb::bson::doc;
use mongodb::{Collection, Cursor};

type IpsInDaterange = Vec<String>;

pub fn time_str_to_bson(
    start_str: &str,
    end_str: &str,
) -> anyhow::Result<(bson::DateTime, bson::DateTime)> {
    let start_utc: Logdate = start_str.parse()?;
    let end_utc: Logdate = end_str.parse()?;
    let s: bson::DateTime = start_utc.into();
    let e: bson::DateTime = end_utc.into();
    Ok((s, e))
}

#[allow(unused_variables)]
pub async fn find_hostdata_by_time_and_country(
    coll: &Collection<HostData>,
    start_str: &str,
    end_str: &str,
    country: &str,
) -> anyhow::Result<Cursor<HostData>> {
    let (start, end) = time_str_to_bson(start_str, end_str)?;
    let filter = doc! {"geodata.country_name": country};
    Ok(coll.find(filter, None).await?)
}

pub async fn find_logentries_by_ip_in_daterange(
    logents_coll: &Collection<LogEntry>,
    ip: &str,
    start_b: DateTime,
    end_b: DateTime,
) -> Cursor<LogEntry> {
    let filter = doc! {"ip" : ip, "time": {"$gte": start_b, "$lte": end_b}};
    logents_coll.find(filter, None).await.unwrap()
}

async fn get_unique_ips_in_daterange(
    coll: &Collection<LogEntry>,
    start_b: DateTime,
    end_b: DateTime,
) -> anyhow::Result<Cursor<Document>> {
    let time_filter = doc! {"$match": {"time": {"$gte": start_b, "$lt": end_b}}};
    let grouper = doc! {"$group": {"_id": "$ip"}};
    let pipeline = vec![time_filter, grouper];
    coll.aggregate(pipeline, None)
        .await
        .map_err(anyhow::Error::msg)
}

pub async fn find_ips_in_daterange(
    coll: &Collection<LogEntry>,
    start_str: &str,
    end_str: &str,
) -> anyhow::Result<IpsInDaterange> {
    let (start_b, end_b) = time_str_to_bson(start_str, end_str)?;
    let mut cursor = get_unique_ips_in_daterange(coll, start_b, end_b).await?;
    let mut ips_in_daterange: IpsInDaterange = vec![];
    while let Some(doc) = cursor.next().await {
        let ndoc = doc?;
        let id = ndoc.get("_id");
        match id {
            Some(Bson::String(id)) => {
                ips_in_daterange.push(id.clone());
            }
            Some(_) => ips_in_daterange.push("type error decoding ip".to_string()),
            None => ips_in_daterange.push("error decoding ip".to_string()),
        }
    }
    Ok(ips_in_daterange)
}
