pub mod json_builder;

use std::sync::Arc;
use axum::{extract::State, response::{IntoResponse, Response}, routing::get, Router, http::{header, StatusCode}};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::aircraft::Store;

/// Serve pre-built JSON from cache — zero serialization on request
async fn aircraft_json(State(store): State<Arc<Store>>) -> Response {
    let body = store.json_cache.read().clone();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache")],
        body,
    ).into_response()
}

async fn receiver_json() -> Response {
    let body = r#"{"refresh":1000,"history":0,"readsb":true,"dbServer":true,"haveTraces":false,"globeIndexGrid":3,"globeIndexSpecialTiles":[],"reapi":false,"binCraft":false,"zstd":false,"version":"skylink-core 0.2.0 (Rust)"}"#;
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache")],
        body,
    ).into_response()
}

async fn stats(State(store): State<Arc<Store>>) -> Response {
    let total = store.map.len();
    let with_pos = store.map.iter().filter(|e| e.value().lat.is_some()).count();
    let msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed);
    let body = format!(r#"{{"aircraft_total":{},"aircraft_with_pos":{},"messages_total":{}}}"#, total, with_pos, msgs);
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}

pub async fn serve(store: Arc<Store>, port: u16) {
    let app = Router::new()
        .route("/data/aircraft.json", get(aircraft_json))
        .route("/data/receiver.json", get(receiver_json))
        .route("/stats", get(stats))
        .layer(CorsLayer::permissive())
        .with_state(store);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("failed to bind API port");
    info!("API on :{}", port);
    axum::serve(listener, app).await.unwrap();
}
