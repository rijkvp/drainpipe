use crate::{
    config::Source,
    daemon::State,
    db::Database,
    dl::{Media, MediaEntry},
    error::Error,
};
use axum::{
    body::{boxed, Full},
    http::{header, StatusCode, Uri},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use reqwest::Url;
use rust_embed::RustEmbed;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tracing::info;

pub fn start(port: u16, db: Arc<Database>, state: Arc<Mutex<State>>) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting web interface on: http://{addr}");

    let app = Router::new()
        .route("/library", get(library))
        .route("/sources", post(set_sources))
        .route("/sources", get(get_sources))
        .route("/tasks", get(get_tasks))
        .route("/queue", get(get_queue))
        .route("/yt_feed", post(yt_feed))
        .fallback(handler)
        .layer(Extension(db))
        .layer(Extension(state));

    tokio::spawn(async move {
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
}

#[derive(RustEmbed)]
#[folder = "gui/"]
struct StaticFile;

async fn handler(uri: Uri) -> Response {
    let mut path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        path = "index.html";
    }
    match StaticFile::get(path) {
        Some(content) => {
            let body = boxed(Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", "/")
            .body(boxed(Full::default()))
            .unwrap(),
    }
}

async fn get_tasks(Extension(state): Extension<Arc<Mutex<State>>>) -> Json<Vec<MediaEntry>> {
    Json(
        state
            .lock()
            .await
            .dl_tasks
            .iter()
            .map(|(l, _)| l.clone())
            .collect(),
    )
}

async fn get_queue(Extension(state): Extension<Arc<Mutex<State>>>) -> Json<Vec<MediaEntry>> {
    let queue = state.lock().await.dl_queue.clone();
    Json(Vec::from_iter(queue.into_iter()))
}

async fn set_sources(
    Extension(state): Extension<Arc<Mutex<State>>>,
    Json(sources): Json<Vec<Source>>,
) -> Result<StatusCode, Error> {
    state
        .lock()
        .await
        .sources
        .set(sources)
        .map(|_| StatusCode::OK)
}

async fn get_sources(Extension(state): Extension<Arc<Mutex<State>>>) -> Json<Vec<Source>> {
    Json(state.lock().await.sources.get())
}

async fn library(Extension(db): Extension<Arc<Database>>) -> Result<Json<Vec<Media>>, Error> {
    db.get_all().await.map(Json)
}

#[derive(Deserialize)]
struct IdRequest {
    url: String,
}

async fn yt_feed(Json(req): Json<IdRequest>) -> Result<String, Error> {
    let url = Url::parse(&req.url)?;
    if url.host_str() != Some("www.youtube.com") {
        return Err(Error::Custom(format!("Invalid host: {:?}", url.host_str())));
    }
    let response = reqwest::get(url).await?.error_for_status()?;
    let text = response.text().await?;
    let html = Html::parse_fragment(&text);
    let selector = Selector::parse("meta").unwrap();
    for element in html.select(&selector) {
        if element.value().attr("itemprop") == Some("channelId") {
            if let Some(id) = element.value().attr("content") {
                return Ok(format!(
                    "https://youtube.com/feeds/videos.xml?channel_id={}",
                    id
                ));
            }
        }
    }
    Err(Error::Custom("Failed to get channel_id!".to_string()))
}
