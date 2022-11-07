use crate::{config::Source, daemon::State, db::Database, dl::Media, error::Error};
use axum::{
    body::{boxed, Full},
    http::{header, StatusCode, Uri},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use log::info;
use parking_lot::Mutex;
use rust_embed::RustEmbed;
use std::{net::SocketAddr, sync::Arc};

pub fn start(port: u16, db: Arc<Database>, state: Arc<Mutex<State>>) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting web interface on: http://{addr}");

    let app = Router::new()
        .route("/library", get(library))
        .route("/sources", post(set_sources))
        .route("/sources", get(get_sources))
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
    match StaticFile::get(&path) {
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

async fn set_sources(
    Extension(state): Extension<Arc<Mutex<State>>>,
    Json(sources): Json<Vec<Source>>,
) -> Result<StatusCode, Error> {
    state.lock().sources.set(sources).map(|_| StatusCode::OK)
}

async fn get_sources(Extension(state): Extension<Arc<Mutex<State>>>) -> Json<Vec<Source>> {
    Json(state.lock().sources.get().clone())
}

async fn library(Extension(db): Extension<Arc<Database>>) -> Result<Json<Vec<Media>>, Error> {
    db.get_all().await.map(|l| Json(l))
}
