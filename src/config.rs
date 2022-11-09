use crate::{dl::MediaEntry, error::Error};
use chrono::{prelude::*, Duration};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigData {
    pub sync_interval: u64,
    pub parallel_downloads: u64,
    pub media_dir: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_filter: Option<DownloadFilter>,
    pub port: u16,
    pub address: IpAddr,
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            sync_interval: 900,
            parallel_downloads: 1,
            media_dir: dirs::home_dir().unwrap().join("media"),
            download_filter: None,
            port: 9193,
            address: Ipv4Addr::UNSPECIFIED.into(),
        }
    }
}

pub struct Config {
    path: PathBuf,
    from_env: bool,
    pub data: ConfigData,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let (data, from_env) = if path.exists() {
            info!("Loading config from file path");
            (crate::file::load(path)?, false)
        } else {
            info!("Loading config from env");
            (envy::prefixed("DRAINPIPE_").from_env::<ConfigData>()?, true)
        };

        Ok(Self {
            path: path.to_path_buf(),
            from_env,
            data,
        })
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        if !self.from_env {
            self.data = crate::file::load(&self.path)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadFilter {
    #[serde(with = "crate::file::dhms_duration_option")]
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
        let sources = crate::file::load_or_create::<Vec<Source>>(&path)?;
        Ok(Self {
            path: path.to_path_buf(),
            sources,
        })
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        self.sources = crate::file::load::<Vec<Source>>(&self.path)?;
        Ok(())
    }

    pub fn get(&self) -> Vec<Source> {
        self.sources.clone()
    }

    pub fn set(&mut self, sources: Vec<Source>) -> Result<(), Error> {
        self.sources = sources;
        crate::file::save(&self.sources, &self.path)?;
        Ok(())
    }
}
