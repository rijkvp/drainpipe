use crate::{dl::MediaEntry, error::Error};
use chrono::{prelude::*, Duration};
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub sync_interval: u64,
    pub parallel_downloads: u64,
    pub media_dir: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_filter: Option<DownloadFilter>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sync_interval: 900,
            parallel_downloads: 1,
            media_dir: dirs::home_dir().unwrap().join("media"),
            download_filter: Some(DownloadFilter::default()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadFilter {
    #[serde(with = "dhms_duration_option")]
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
                if (Utc::now() - published) > max_age {
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

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Sources {
    pub sources: Vec<String>,
}

pub trait Loadable<T: Sized> {
    fn load(path: &Path) -> Result<T, Error>;
    fn reload(path: &Path) -> Result<T, Error>;
}

impl<T> Loadable<T> for T
where
    T: Default + for<'a> Deserialize<'a> + Serialize,
{
    fn load(path: &Path) -> Result<T, Error> {
        if path.exists() {
            info!("Loading config {path:?}");
            let config_str = fs::read_to_string(path)?;
            Ok(serde_yaml::from_str(&config_str)?)
        } else {
            let config = T::default();
            info!("Default file generated at {path:?}");
            let config_str = serde_yaml::to_string(&config)?;
            fs::write(path, config_str)?;
            Ok(config)
        }
    }

    fn reload(path: &Path) -> Result<T, Error> {
        info!("Reloading {path:?}");
        let config_str = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str::<T>(&config_str)?)
    }
}

mod dhms_duration {
    use chrono::Duration;
    use serde::{de::Error, Deserialize, Deserializer, Serializer};
    use std::num::ParseIntError;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        fn format(n: i64, c: &str) -> String {
            if n > 0 {
                n.to_string() + c
            } else {
                String::new()
            }
        }
        serializer.serialize_str(
            &[
                format(duration.num_days(), "d"),
                format(duration.num_hours() % 24, "h"),
                format(duration.num_minutes() % 60, "m"),
                format(duration.num_seconds() % 60, "s"),
            ]
            .concat(),
        )
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        fn parse(c: char, s: &str, p: &mut usize) -> Result<i64, ParseIntError> {
            if let Some(i) = s.find(c) {
                let r = s[*p..i].parse();
                *p = i + 1;
                r
            } else {
                Ok(0)
            }
        }
        let mut p = 0;
        let d = parse('d', &str, &mut p).map_err(Error::custom)?;
        let h = parse('h', &str, &mut p).map_err(Error::custom)?;
        let m = parse('m', &str, &mut p).map_err(Error::custom)?;
        let s = parse('s', &str, &mut p).map_err(Error::custom)?;
        Ok(Duration::seconds(
            d * 24 * 60 * 60 + h * 60 * 60 + m * 60 + s,
        ))
    }
}

mod dhms_duration_option {
    use super::dhms_duration;
    use chrono::Duration;
    use serde::{de::Error, Deserializer, Serializer};

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(dur) => dhms_duration::serialize(dur, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match dhms_duration::deserialize(deserializer) {
            Ok(dur) => Ok(Some(dur)),
            Err(err) => Err(Error::custom(err)),
        }
    }
}
