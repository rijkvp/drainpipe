use crate::{error::Error, source::SourceType};
use chrono::prelude::*;
use feed_rs::model::Entry as FeedEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct MediaEntry {
    pub title: Option<String>,
    pub link: String,
    pub published: Option<DateTime<Utc>>,
    pub r#type: SourceType,
}

impl PartialEq for MediaEntry {
    fn eq(&self, other: &Self) -> bool {
        self.link == other.link
    }
}

impl MediaEntry {
    pub fn from_feed_entry(e: FeedEntry, r#type: SourceType) -> Result<Self, Error> {
        Ok(Self {
            title: e.title.map(|t| t.content),
            published: e.published,
            link: e
                .links
                .get(0)
                .ok_or_else(|| Error::Custom("No link on entry!".to_string()))?
                .href
                .clone(),
            r#type,
        })
    }
}

#[derive(sqlx::FromRow, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Media {
    #[serde(alias = "webpage_url")]
    pub source: String,
    pub id: String,
    #[serde(alias = "filename")]
    pub path: String,
    pub title: String,
    pub description: String,
}
