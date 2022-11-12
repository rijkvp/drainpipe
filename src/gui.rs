use crate::{
    config::ConfigData,
    daemon::State,
    db::Database,
    error::Error,
    media::{Media, MediaEntry},
    source::Source,
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
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tracing::info;

pub fn start(port: u16, db: Arc<Database>, state: Arc<Mutex<State>>) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting web interface on: http://{addr}");

    let app = Router::new()
        .route("/sources", post(set_sources))
        .route("/sources", get(get_sources))
        .route("/state", get(get_state))
        .route("/config", get(get_config))
        .route("/config", post(set_config))
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

#[derive(Serialize)]
struct StateResponse {
    tasks: Vec<MediaEntry>,
    queue: Vec<MediaEntry>,
    library: Vec<Media>,
}

async fn get_state(
    Extension(state): Extension<Arc<Mutex<State>>>,
    Extension(db): Extension<Arc<Database>>,
) -> Result<Json<StateResponse>, Error> {
    let state = state.lock().await;
    let tasks = state.dl_tasks.iter().map(|(l, _)| l.clone()).collect();
    let queue = Vec::from_iter(state.dl_queue.clone().into_iter());
    let library = db.get_all().await?;

    Ok(Json(StateResponse {
        tasks,
        queue,
        library,
    }))
}

async fn get_sources(Extension(state): Extension<Arc<Mutex<State>>>) -> Json<Vec<Source>> {
    Json(state.lock().await.sources.get())
}

async fn set_sources(
    Extension(state): Extension<Arc<Mutex<State>>>,
    Json(sources): Json<Vec<Source>>,
) -> Result<(), Error> {
    state.lock().await.sources.set(sources)
}

async fn get_config(Extension(state): Extension<Arc<Mutex<State>>>) -> Json<ConfigData> {
    Json(state.lock().await.config.data.clone())
}

async fn set_config(
    Extension(state): Extension<Arc<Mutex<State>>>,
    Json(config): Json<ConfigData>,
) -> Result<(), Error> {
    state.lock().await.config.set(config)
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
