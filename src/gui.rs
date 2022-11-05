use axum::{
    body::{boxed, Full},
    http::{header, StatusCode, Uri},
    response::Response,
    routing::get,
    Extension, Json, Router,
};
use log::{error, info};
use rust_embed::RustEmbed;
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};

use crate::db::Database;

pub fn start(port: u16, db: Arc<Database>) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Starting web interface on: http://{addr}");

    let app = Router::new()
        .route("/library", get(library))
        .fallback(handler)
        .layer(Extension(db));
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

async fn library(Extension(db): Extension<Arc<Database>>) -> Result<Json<Value>, StatusCode> {
    db.get_all().await.map(|a| Json(json!(a))).map_err(|e| {
        error!("Failed database query: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
