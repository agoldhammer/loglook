// use super::HostData;
use super::Logdate;
use crate::log_entries::LogEntry;
use bson;
use bson::Document;
use bson::{Bson, DateTime};
use futures::stream::{StreamExt, TryStreamExt};
use mongodb::bson::doc;
// use mongodb::options::FindOptions;
use mongodb::{Collection, Cursor};
use serde::{Deserialize, Serialize};

type IpsInDaterange = Vec<String>;

#[derive(Debug)]
pub struct DateRange {
    pub start: DateTime,
    pub end: DateTime,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CountryWithIps {
    #[serde(alias = "_id")]
    pub country: String,
    pub ips: Vec<String>,
}

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

pub fn time_str_to_daterange(start_str: &str, end_str: &str) -> anyhow::Result<DateRange> {
    let (s, e) = time_str_to_bson(start_str, end_str)?;
    Ok(DateRange { start: s, end: e })
}

pub async fn find_logentries_by_ip_in_daterange(
    logents_coll: &Collection<LogEntry>,
    ip: &str,
    date_range: &DateRange,
) -> anyhow::Result<Cursor<LogEntry>> {
    let filter = doc! {"ip" : ip, "time": {"$gte": date_range.start, "$lte": date_range.end}};
    logents_coll
        .find(filter, None)
        .await
        .map_err(anyhow::Error::msg)
}

async fn get_unique_ips_in_daterange(
    coll: &Collection<LogEntry>,
    date_range: &DateRange,
) -> anyhow::Result<Cursor<Document>> {
    let time_filter = doc! {"$match": {"time": {"$gte": date_range.start, "$lt": date_range.end}}};
    let grouper = doc! {"$group": {"_id": "$ip"}};
    let sorter = doc! {"$sort": {"_id": 1}};
    let pipeline = vec![time_filter, grouper, sorter];
    coll.aggregate(pipeline, None)
        .await
        .map_err(anyhow::Error::msg)
}

pub async fn find_ips_in_daterange(
    coll: &Collection<LogEntry>,
    date_range: &DateRange,
) -> anyhow::Result<IpsInDaterange> {
    let mut cursor = get_unique_ips_in_daterange(coll, date_range).await?;
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

pub async fn make_current_le_coll(
    date_range: &DateRange,
    logentry_coll: &Collection<LogEntry>,
) -> anyhow::Result<()> {
    let time_filter = doc! {"$match": {"time": {"$gte": date_range.start, "$lt": date_range.end}}};
    let out_coll = doc! {"$out": "current_logentries"};
    let _ = logentry_coll
        .aggregate(vec![time_filter, out_coll], None)
        .await?;
    Ok(())
}

// * must call make_current_le_coll before calling this!
pub async fn get_current_ips_by_country(
    current_logentries_coll: &Collection<LogEntry>,
) -> anyhow::Result<Vec<CountryWithIps>> {
    let pipeline = [
        doc! {
            "$lookup": doc! {
                "as": "hostdata",
                "from": "hostdata",
                "foreignField": "ip",
                "localField": "ip"
            }
        },
        doc! {
            "$project": doc! {
                "ip": 1,
                "hostdata.geodata.country_name": 1
            }
        },
        doc! {
            "$sort": doc! {
                "hostdata.geodata.country_name": 1
            }
        },
        doc! {
            "$set": doc! {
                "country": "$hostdata.geodata.country_name"
            }
        },
        doc! {
            "$project": doc! {
                "hostdata": 0
            }
        },
        doc! {
            "$unwind": doc! {
                "path": "$country",
                "preserveNullAndEmptyArrays": false
            }
        },
        doc! {
            "$group": doc! {
                "_id": "$country",
                "ips": doc! {
                    "$addToSet": "$ip"
                }
            }
        },
        doc! {"$sort": doc! {"_id": 1}},
    ];
    let curs = current_logentries_coll.aggregate(pipeline, None).await?;
    let docs = curs.try_collect::<Vec<Document>>().await?;
    let mut country_with_ip_list: Vec<CountryWithIps> = vec![];
    for doc in docs {
        let vip: CountryWithIps = bson::from_document(doc)?;
        country_with_ip_list.push(vip);
    }
    Ok(country_with_ip_list)
}
