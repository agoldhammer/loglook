// use super::HostData;
use super::Logdate;
use crate::log_entries::LogEntry;
use bson;
use bson::Document;
use bson::{Bson, DateTime};
use futures::stream::StreamExt;
use mongodb::bson::doc;
// use mongodb::options::FindOptions;
use mongodb::{Collection, Cursor};

type IpsInDaterange = Vec<String>;

#[derive(Debug)]
pub struct DateRange {
    pub start: DateTime,
    pub end: DateTime,
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

#[allow(dead_code)]
#[allow(unused_variables)]
pub async fn find_ips_in_daterange_by_country(
    // hdcoll: &Collection<HostData>,
    lecoll: &Collection<LogEntry>,
    date_range: &DateRange,
) -> anyhow::Result<()> {
    let pipeline = [
        doc! {
            "$match": doc! {
                "$and": [
                    doc! {
                        "time": doc! {
                            "$gte": date_range.start,
                        }
                    },
                    doc! {
                        "time": doc! {
                            "$lte": date_range.end,
                        }
                    }
                ]
            }
        },
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
    ];
    let mut curs = lecoll.aggregate(pipeline, None).await?;
    while let Some(doc) = curs.next().await {
        println!("newfn: {:?}", doc);
    }
    Ok(())
}

// #[allow(unused_variables)]
// pub async fn find_hostdata_by_time_and_country(
//     coll: &Collection<HostData>,
//     start_str: &str,
//     end_str: &str,
//     country: &str,
// ) -> anyhow::Result<Cursor<HostData>> {
//     // let (start, end) = time_str_to_bson(start_str, end_str)?;
//     let filter = doc! {"geodata.country_name": country};
//     Ok(coll.find(filter, None).await?)
// }

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
