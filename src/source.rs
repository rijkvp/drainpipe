use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tracing::log::info;
use crate::error::Error;

#[derive(Clone, Serialize, Deserialize)]
pub struct Source {
    pub url: String,
    pub r#type: SourceType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Video,
    Audio,
}

pub struct Sources {
    path: PathBuf,
    sources: Vec<Source>,
    changed: bool,
}

impl Sources {
    pub fn load(path: &Path) -> Result<Self, Error> {
        info!("Loading sources from {path:?}");
        let sources = crate::file::load_or_create::<Vec<Source>>(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            sources,
            changed: false,
        })
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        self.sources = crate::file::load::<Vec<Source>>(&self.path)?;
        self.changed = true;
        Ok(())
    }

    pub fn get(&self) -> Vec<Source> {
        self.sources.clone()
    }

    pub fn set(&mut self, sources: Vec<Source>) -> Result<(), Error> {
        self.sources = sources;
        crate::file::save(&self.sources, &self.path)?;
        self.changed = true;
        Ok(())
    }

    pub fn changed(&mut self) -> bool {
        if self.changed {
            self.changed = false;
            return true;
        }
        false
    }
}
