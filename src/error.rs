use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File watcher error: {0}")]
    FileWatch(#[from] notify::Error),
    #[error("YAML deserialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Env deserialization error: {0}")]
    Env(#[from] envy::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Feed parse error: {0}")]
    Feed(#[from] feed_rs::parser::ParseFeedError),
    #[error("Sqlite error: {0}")]
    Sqlite(#[from] sqlx::Error),
    #[error("{0}")]
    Custom(String),
}

use axum::{
    body::boxed,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let err = self.to_string();
        error!("{err}");
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(boxed(err))
            .unwrap()
    }
}
