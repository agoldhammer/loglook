use super::Logdate;
use crate::log_entries::LogEntry;
use bson;
use bson::Bson;
use bson::Document;
use futures::stream::StreamExt;
use mongodb::bson::doc;
use mongodb::{Collection, Cursor};

type IpsInDaterange = Vec<String>;

async fn get_unique_ips_in_daterange(
    coll: &Collection<LogEntry>,
    start_utc: Logdate,
    end_utc: Logdate,
) -> anyhow::Result<Cursor<Document>> {
    let s: bson::DateTime = start_utc.into();
    let e: bson::DateTime = end_utc.into();
    let time_filter = doc! {"$match": {"time": {"$gte": s, "$lt": e}}};
    let grouper = doc! {"$group": {"_id": "$ip"}};
    let pipeline = vec![time_filter, grouper];
    coll.aggregate(pipeline, None)
        .await
        .map_err(anyhow::Error::msg)
}

pub async fn find_ips_in_daterange(
    coll: &Collection<LogEntry>,
    start_utc: Logdate,
    end_utc: Logdate,
) -> anyhow::Result<IpsInDaterange> {
    let mut cursor = get_unique_ips_in_daterange(coll, start_utc, end_utc).await?;
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
