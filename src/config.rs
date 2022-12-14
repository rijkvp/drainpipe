use crate::{error::Error, media::MediaEntry};
use chrono::{prelude::*, Duration};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigData {
    pub address: IpAddr,
    pub port: u16,
    pub sync_interval: u64,
    pub parallel_downloads: u64,
    pub media_dir: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_filter: Option<DownloadFilter>,
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            address: Ipv4Addr::UNSPECIFIED.into(),
            port: 9193,
            sync_interval: 900,
            parallel_downloads: 1,
            media_dir: dirs::home_dir().unwrap().join("media"),
            download_filter: Some(DownloadFilter::default()),
        }
    }
}

pub struct Config {
    path: PathBuf,
    pub from_env: bool,
    pub data: ConfigData,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let (data, from_env) = if path.exists() {
            info!("Loading config from file path");
            (crate::file::load(path)?, false)
        } else {
            info!("Loading config from environment");
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

    pub fn set(&mut self, data: ConfigData) -> Result<(), Error> {
        self.data = data;
        crate::file::save(&self.data, &self.path)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFilter {
    #[serde(with = "crate::file::dhms_duration_option")]
    pub max_age: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<NaiveDate>,
}

impl DownloadFilter {
    pub fn filter(&self, entry: &MediaEntry) -> bool {
        if let Some(published) = entry.published {
            if let Some(before) = self.before {
                if published
                    > DateTime::<Utc>::from_local(before.and_hms_opt(0, 0, 0).unwrap(), Utc)
                {
                    return true;
                }
            }
            if let Some(after) = self.after {
                if published < DateTime::<Utc>::from_local(after.and_hms_opt(0, 0, 0).unwrap(), Utc)
                {
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
            max_age: Some(Duration::days(30)),
            before: None,
            after: None,
        }
    }
}
