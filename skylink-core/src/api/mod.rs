pub mod json_builder;

use std::sync::Arc;
use axum::{extract::State, response::{IntoResponse, Response}, routing::get, Router, http::{header, StatusCode}};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::aircraft::Store;
use crate::bincraft;

/// Serve pre-built JSON from cache — zero serialization on request
async fn aircraft_json(State(store): State<Arc<Store>>) -> Response {
    let body = store.json_cache.read().clone();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache")],
        body,
    ).into_response()
}

/// Serve binCraft binary format
async fn aircraft_bincraft(State(store): State<Arc<Store>>) -> Response {
    let body = store.bincraft_cache.read().clone();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream"), (header::CACHE_CONTROL, "no-cache")],
        body,
    ).into_response()
}

/// re-api endpoint (tar1090 / ml_clf_fe use this)
/// Supports: ?binCraft, ?binCraft&box=south,north,west,east, ?binCraft&zstd
async fn re_api(State(store): State<Arc<Store>>, axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>) -> Response {
    if params.contains_key("binCraft") {
        let use_zstd = params.contains_key("zstd");
        let raw = if let Some(box_param) = params.get("box") {
            let parts: Vec<f64> = box_param.split(',').filter_map(|s| s.parse().ok()).collect();
            if parts.len() == 4 {
                crate::bincraft::build_filtered(&store, parts[0], parts[1], parts[2], parts[3])
            } else {
                store.bincraft_cache.read().to_vec()
            }
        } else {
            store.bincraft_cache.read().to_vec()
        };

        let body = if use_zstd {
            zstd::encode_all(raw.as_slice(), 1).unwrap_or(raw)
        } else {
            raw
        };

        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/octet-stream"),
             (header::CACHE_CONTROL, "no-cache"),
             (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")],
            body,
        ).into_response();
    }
    aircraft_json(State(store)).await
}

async fn receiver_json() -> Response {
    let body = r#"{"refresh":1000,"history":0,"readsb":true,"dbServer":true,"haveTraces":false,"globeIndexGrid":3,"globeIndexSpecialTiles":[],"reapi":true,"binCraft":true,"zstd":false,"version":"skylink-core 0.2.0 (Rust)"}"#;
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache")],
        body,
    ).into_response()
}

async fn aircraft_pb(State(store): State<Arc<Store>>) -> Response {
    let body = crate::pb::build_aircraft_pb(&store);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-protobuf"), (header::CACHE_CONTROL, "no-cache")],
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
        .route("/data/aircraft.binCraft", get(aircraft_bincraft))
        .route("/data/aircraft.pb", get(aircraft_pb))
        .route("/data/receiver.json", get(receiver_json))
        .route("/re-api/", get(re_api))
        .route("/stats", get(stats))
        .layer(CorsLayer::permissive())
        .with_state(store);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("failed to bind API port");
    info!("API on :{}", port);
    axum::serve(listener, app).await.unwrap();
}
