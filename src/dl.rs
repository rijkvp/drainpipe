use crate::{config::Sources, error::Error};
use chrono::prelude::*;
use feed_rs::{model::Entry as FeedEntry, parser};
use futures::StreamExt;
use log::{debug, error};
use parking_lot::Mutex;
use reqwest::Client;
use serde::Deserialize;
use std::{
    process::Command,
    sync::Arc,
    thread::{self, JoinHandle},
};

#[derive(Debug, Clone)]
pub struct MediaEntry {
    pub title: Option<String>,
    pub link: String,
    pub published: Option<DateTime<Utc>>,
}

impl TryFrom<FeedEntry> for MediaEntry {
    type Error = Error;

    fn try_from(e: FeedEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            title: e.title.map(|t| t.content),
            published: e.published,
            link: e
                .links
                .get(0)
                .ok_or_else(|| Error::Custom("No link on entry!".to_string()))?
                .href
                .clone(),
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MediaFile {
    #[serde(rename = "webpage_url")]
    pub source: String,
    pub id: String,
    #[serde(rename = "filename")]
    pub path: String,
    #[serde(rename = "fulltitle")]
    pub title: String,
    pub description: String,
    // pub channel: String,
    // pub thumbnail: String,
}

pub async fn crawl_sources(sources: Sources) -> Vec<MediaEntry> {
    let client = Client::new();
    let items = Arc::new(Mutex::new(Vec::new()));
    tokio_stream::iter(&sources.sources)
        .for_each_concurrent(128, |source| {
            let client = client.clone();
            let items = items.clone();
            async move {
                match get_feed_downloads(client, source).await {
                    Ok(m) => items.lock().extend(m),
                    Err(e) => error!("Error: {e}"),
                }
            }
        })
        .await;

    let items = items.lock().to_vec();
    debug!(
        "Got {} entries from {} sources",
        items.len(),
        sources.sources.len()
    );
    items
}

async fn get_feed_downloads(client: Client, url: &str) -> Result<Vec<MediaEntry>, Error> {
    let response = client.get(url).send().await?.error_for_status()?;
    let xml = response.text().await?;
    let feed = parser::parse(xml.as_bytes())?;
    let mut items = Vec::new();
    for entry in feed.entries {
        let dl = MediaEntry::try_from(entry)?;
        items.push(dl);
    }
    debug!("Feed: got {} entries from {}", items.len(), url);
    Ok(items)
}

pub fn download_video(dir: String, url: String) -> JoinHandle<Result<MediaFile, String>> {
    thread::spawn(move || -> Result<MediaFile, String> {
        let output = Command::new("yt-dlp")
            .args([
                "-f",
                "(bv[vcodec^=vp9][height<=1080]/bv[height<=1080]/bv)+(ba[acodec=opus]/ba/b)",
                "--merge-output-format",
                "mkv",
                "--print",
                "%()j",
                "--no-simulate",
                "--no-progress",
                "-o",
                &(dir + "/%(artist,channel,uploader|Unkown)s/%(release_date>%Y%m%d,upload_date>%Y%m%d)s-%(fulltitle)s.%(ext)s"),
                &url,
            ])
            .output()
            .map_err(|e| format!("Failed execute yt-dlp: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if output.status.success() {
            let info = serde_json::from_str::<MediaFile>(&stdout).map_err(|e| {
                format!(
                    "Failed to parse JSON: {e}\nSource: {stdout}\n^^^ End failed to parse JSON."
                )
            })?;
            Ok(info)
        } else {
            Err(format!("YT-DLP failed:\n{stderr}\n{stdout}",))
        }
    })
}
