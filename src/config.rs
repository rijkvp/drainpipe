use crate::{dl::MediaEntry, error::Error, file};
use chrono::{prelude::*, Duration};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub sync_interval: u64,
    pub parallel_downloads: u64,
    pub media_dir: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_filter: Option<DownloadFilter>,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sync_interval: 900,
            parallel_downloads: 1,
            media_dir: dirs::home_dir().unwrap().join("media"),
            download_filter: None,
            port: 9193,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadFilter {
    #[serde(with = "file::dhms_duration_option")]
    pub max_age: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<DateTime<Utc>>,
}

impl DownloadFilter {
    pub fn filter(&self, entry: &MediaEntry) -> bool {
        if let Some(published) = entry.published {
            if let Some(before) = self.before {
                if published > before {
                    return true;
                }
            }
            if let Some(after) = self.after {
                if published < after {
                    return true;
                }
            }
            if let Some(max_age) = self.max_age {
                if Utc::now() - published > max_age {
                    return true;
                }
            }
        }
        false
    }
}

impl Default for DownloadFilter {
    fn default() -> Self {
        Self {
            max_age: Some(Duration::days(7)),
            before: None,
            after: None,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Video,
    Audio,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Source {
    pub url: String,
    pub r#type: SourceType,
}

pub struct Sources {
    path: PathBuf,
    sources: Vec<Source>,
}

impl Sources {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let sources = file::load_or_create::<Vec<Source>>(&path)?;
        Ok(Self {
            path: path.to_path_buf(),
            sources,
        })
    }

    pub fn reload(&mut self, path: &Path) -> Result<(), Error> {
        self.sources = file::load::<Vec<Source>>(&path)?;
        Ok(())
    }

    pub fn get(&self) -> Vec<Source> {
        self.sources.clone()
    }

    pub fn set(&mut self, sources: Vec<Source>) -> Result<(), Error> {
        self.sources = sources;
        file::save(&self.sources, &self.path)?;
        Ok(())
    }
}
