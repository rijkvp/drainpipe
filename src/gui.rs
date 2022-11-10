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
use rust_embed::RustEmbed;
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
        (&state.lock().await.dl_tasks)
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
