pub mod json_builder;

use std::sync::Arc;
use axum::{extract::State, response::{IntoResponse, Response}, routing::get, Router, http::{header, StatusCode}};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::aircraft::Store;

async fn aircraft_compact(State(store): State<Arc<Store>>) -> Response {
    let raw = crate::compact::build(&store);
    let body = zstd::encode_all(raw.as_slice(), 3).unwrap_or(raw);
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/octet-stream"), (header::CACHE_CONTROL, "no-cache")], body).into_response()
}

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

/// re-api endpoint — unified query interface for all 3 formats
/// ?binCraft, ?pb, ?json (default)
/// Optional: &zstd, &box=south,north,west,east
async fn re_api(State(store): State<Arc<Store>>, axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>) -> Response {
    let use_zstd = params.contains_key("zstd");
    let bbox = params.get("box").and_then(|b| {
        let p: Vec<f64> = b.split(',').filter_map(|s| s.parse().ok()).collect();
        if p.len() == 4 { Some((p[0], p[1], p[2], p[3])) } else { None }
    });

    if params.contains_key("compact") {
        let (raw, cached) = match bbox {
            Some((s, n, w, e)) => (crate::compact::build_filtered(&store, s, n, w, e), None),
            None => (vec![], Some(if use_zstd { store.compact_zstd_cache.read().clone() } else { store.compact_cache.read().clone() })),
        };
        if let Some(c) = cached {
            return (StatusCode::OK, [(header::CONTENT_TYPE, "application/octet-stream"), (header::CACHE_CONTROL, "no-cache"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], c).into_response();
        }
        return serve_binary(raw, use_zstd, "application/octet-stream");
    }

    if params.contains_key("binCraft") {
        let (raw, zstd_cached) = match bbox {
            Some((s, n, w, e)) => (crate::bincraft::build_filtered(&store, s, n, w, e), None),
            None => (vec![], Some(if use_zstd { store.bincraft_zstd_cache.read().clone() } else { store.bincraft_cache.read().clone() })),
        };
        if let Some(cached) = zstd_cached {
            return (StatusCode::OK, [(header::CONTENT_TYPE, "application/octet-stream"), (header::CACHE_CONTROL, "no-cache"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], cached).into_response();
        }
        return serve_binary(raw, use_zstd, "application/octet-stream");
    }

    if params.contains_key("pb") {
        let (raw, zstd_cached) = match bbox {
            Some((s, n, w, e)) => (crate::pb::build_filtered(&store, s, n, w, e), None),
            None => (vec![], Some(if use_zstd { store.pb_zstd_cache.read().clone() } else { store.pb_cache.read().clone() })),
        };
        if let Some(cached) = zstd_cached {
            return (StatusCode::OK, [(header::CONTENT_TYPE, "application/x-protobuf"), (header::CACHE_CONTROL, "no-cache"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], cached).into_response();
        }
        return serve_binary(raw, use_zstd, "application/x-protobuf");
    }

    // JSON (default)
    let (raw, zstd_cached) = match bbox {
        Some((s, n, w, e)) => (crate::aircraft::build_json_filtered(&store, s, n, w, e), None),
        None => (vec![], Some(if use_zstd { store.json_zstd_cache.read().clone() } else { store.json_cache.read().clone() })),
    };
    if let Some(cached) = zstd_cached {
        return (StatusCode::OK, [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], cached).into_response();
    }
    serve_binary(raw, use_zstd, "application/json")
}

fn serve_binary(raw: Vec<u8>, use_zstd: bool, content_type: &'static str) -> Response {
    let body = if use_zstd {
        zstd::encode_all(raw.as_slice(), 1).unwrap_or(raw)
    } else {
        raw
    };
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type),
         (header::CACHE_CONTROL, "no-cache"),
         (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")],
        body,
    ).into_response()
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
    let body = store.pb_cache.read().clone();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-protobuf"), (header::CACHE_CONTROL, "no-cache")],
        body,
    ).into_response()
}

async fn aircraft_pb_zstd(State(store): State<Arc<Store>>) -> Response {
    let raw = store.pb_cache.read().clone();
    let body = zstd::encode_all(raw.as_ref(), 1).unwrap_or(raw.to_vec());
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-protobuf"),
         (header::CONTENT_ENCODING, "zstd"),
         (header::CACHE_CONTROL, "no-cache")],
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
        .route("/data/aircraft.pb.zst", get(aircraft_pb_zstd))
        .route("/data/receiver.json", get(receiver_json))
        .route("/re-api/", get(re_api))
        .route("/data/aircraft.compact", get(aircraft_compact))
        .route("/ws", get(crate::ws::ws_handler))
        .route("/stats", get(stats))
        .layer(CorsLayer::permissive())
        .with_state(store);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("failed to bind API port");
    info!("API on :{}", port);
    axum::serve(listener, app).await.unwrap();
}
