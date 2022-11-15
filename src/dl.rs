use crate::{
    error::Error,
    media::{Media, MediaEntry},
    source::{Source, SourceType},
};
use chrono::Utc;
use feed_rs::parser;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    process::Command,
    sync::Arc,
    thread::{self, JoinHandle},
};
use tokio::sync::Mutex;
use tracing::{debug, error};

pub async fn crawl_sources(sources: Vec<Source>) -> Vec<MediaEntry> {
    debug!("Crawling feeds from {} sources..", sources.len());
    let client = Client::new();
    let items = Arc::new(Mutex::new(Vec::new()));
    tokio_stream::iter(&sources)
        .for_each_concurrent(128, |source| {
            let client = client.clone();
            let items = items.clone();
            async move {
                match get_source_entries(client, source).await {
                    Ok(m) => items.lock().await.extend(m),
                    Err(e) => error!("Failed to get downloads for '{}': {e}", source.url),
                }
            }
        })
        .await;

    let items = items.lock().await.to_vec();
    debug!("Got {} entries from {} sources", items.len(), sources.len());
    items
}

async fn get_source_entries(client: Client, source: &Source) -> Result<Vec<MediaEntry>, Error> {
    let response = client.get(&source.url).send().await?.error_for_status()?;
    let xml = response.text().await?;
    let feed = parser::parse(xml.as_bytes())?;
    let mut items = Vec::new();
    for entry in feed.entries {
        let dl = MediaEntry::from_feed_entry(entry, source.r#type.clone())?;
        items.push(dl);
    }
    debug!("Feed: got {} entries from {}", items.len(), source.url);
    Ok(items)
}

fn dl_format(dl_type: SourceType) -> Vec<&'static str> {
    match dl_type {
        SourceType::Video => {
            vec![
                "-f",
                "(bv[vcodec^=vp9][height<=1080]/bv[height<=1080]/bv)+(ba[acodec=opus]/ba/b)",
                "--merge-output-format",
                "mkv",
            ]
        }
        SourceType::Audio => vec![
            "-f",
            "ba[acodec=opus]/ba/b",
            "--extract-audio",
            "--audio-format",
            "opus",
        ],
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DownloadInfo {
    #[serde(alias = "webpage_url")]
    pub source: String,
    pub id: String,
    #[serde(alias = "filename")]
    pub path: String,
    pub title: String,
    pub description: String,
}

impl DownloadInfo {
    fn into_media(self) -> Media {
        Media {
            source: self.source,
            id: self.id,
            path: self.path,
            title: self.title,
            description: self.description, 
            date: Utc::now().timestamp(),
        }
    }
}

pub fn download_video(dir: String, entry: MediaEntry) -> JoinHandle<Result<Media, String>> {
    thread::spawn(move || -> Result<Media, String> {
        let output = Command::new("yt-dlp")
            .args([
                dl_format(entry.r#type),
                vec![
                    "--embed-thumbnail",
                    "--embed-metadata",
                    "--embed-info-json",
                    "--print",
                    "%()j",
                    "--no-simulate",
                    "--no-progress",
                    "-o",
                    &(dir + "/%(artist,channel,uploader|Unkown)s/%(release_date>%Y%m%d,upload_date>%Y%m%d)s-%(fulltitle)s.%(ext)s"),
                    &entry.link,
                ]
            ].concat())
            .output()
            .map_err(|e| format!("Failed execute yt-dlp: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if output.status.success() {
            let info = serde_json::from_str::<DownloadInfo>(&stdout)
                .map_err(|e| format!("Failed to parse JSON: {e}"))?;
            Ok(info.into_media())
        } else {
            Err(format!("YT-DLP failed:\n{stderr}\n{stdout}",))
        }
    })
}
