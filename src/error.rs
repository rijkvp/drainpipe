use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File watcher error: {0}")]
    FileWatch(#[from] notify::Error),
    #[error("YAML deserialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Feed parse error: {0}")]
    Feed(#[from] feed_rs::parser::ParseFeedError),
    #[error("Sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("{0}")]
    Custom(String),
}
