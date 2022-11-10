use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use tracing::debug;

pub fn load_or_create<T>(path: &Path) -> Result<T, Error>
where
    T: Default + for<'a> Deserialize<'a> + Serialize,
{
    if path.exists() {
        load(path)
    } else {
        let default = T::default();
        let contents = serde_yaml::to_string(&default)?;
        fs::write(path, contents)?;
        debug!("Default file generated at {path:?}");
        Ok(default)
    }
}

pub fn load<T>(path: &Path) -> Result<T, Error>
where
    T: Default + for<'a> Deserialize<'a> + Serialize,
{
    debug!("Reloading file {path:?}");
    let contents = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str::<T>(&contents)?)
}

pub fn save<T>(data: &T, path: &Path) -> Result<(), Error>
where
    T: Default + for<'a> Deserialize<'a> + Serialize,
{
    debug!("Saving file {path:?}");
    let contents = serde_yaml::to_string(data)?;
    fs::write(path, contents)?;
    Ok(())
}

pub mod dhms_duration {
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

pub mod dhms_duration_option {
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
