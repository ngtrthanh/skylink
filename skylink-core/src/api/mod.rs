pub mod json_builder;

use std::sync::Arc;
use std::collections::HashMap;
use axum::{extract::{State, Path, Query}, response::{IntoResponse, Response}, routing::get, Router, http::{header, StatusCode}};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::aircraft::Store;

// --- Cached endpoints (pre-built every 1s) ---

async fn aircraft_json(State(s): State<Arc<Store>>) -> Response { serve_cached(s.json_cache.read().clone(), "application/json") }
async fn aircraft_bincraft(State(s): State<Arc<Store>>) -> Response { serve_cached(s.bincraft_cache.read().clone(), "application/octet-stream") }
async fn aircraft_pb(State(s): State<Arc<Store>>) -> Response { serve_cached(s.pb_cache.read().clone(), "application/x-protobuf") }
async fn aircraft_pb_zstd(State(s): State<Arc<Store>>) -> Response { serve_cached(s.pb_zstd_cache.read().clone(), "application/x-protobuf") }
async fn aircraft_compact(State(s): State<Arc<Store>>) -> Response { serve_cached(s.compact_zstd_cache.read().clone(), "application/octet-stream") }

fn serve_cached(body: bytes::Bytes, ct: &'static str) -> Response {
    (StatusCode::OK, [(header::CONTENT_TYPE, ct), (header::CACHE_CONTROL, "no-cache")], body).into_response()
}

// --- aircraft_recent.json: only aircraft seen in last 60s ---

async fn aircraft_recent(State(store): State<Arc<Store>>) -> Response {
    let now = now_secs();
    let mut buf = Vec::with_capacity(256 * 1024);
    buf.extend_from_slice(b"{\"now\":");
    buf.extend_from_slice(format!("{:.1}", now).as_bytes());
    buf.extend_from_slice(b",\"aircraft\":[");
    let mut first = true;
    for entry in store.map.iter() {
        let ac = entry.value();
        if now - ac.last_update < 60.0 {
            if !first { buf.push(b','); }
            first = false;
            buf.extend_from_slice(&serde_json::to_vec(ac).unwrap_or_default());
        }
    }
    buf.extend_from_slice(b"]}");
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache")], buf).into_response()
}

// --- status.json: decoder statistics ---

async fn status_json(State(store): State<Arc<Store>>) -> Response {
    let total = store.map.len();
    let with_pos = store.map.iter().filter(|e| e.value().lat.is_some()).count();
    let msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed);
    let now = now_secs();
    let body = format!(r#"{{"now":{:.1},"aircraft_count":{},"aircraft_count_with_pos":{},"messages_total":{},"uptime":{:.0},"version":"skylink-core 0.3.0"}}"#,
        now, total, with_pos, msgs, now - store.start_time);
    json_response(body)
}

// --- status.prom: Prometheus metrics ---

async fn status_prom(State(store): State<Arc<Store>>) -> Response {
    let total = store.map.len();
    let with_pos = store.map.iter().filter(|e| e.value().lat.is_some()).count();
    let msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed);
    let body = format!(
        "# HELP skylink_aircraft_total Total tracked aircraft\n# TYPE skylink_aircraft_total gauge\nskylink_aircraft_total {}\n\
         # HELP skylink_aircraft_with_pos Aircraft with position\n# TYPE skylink_aircraft_with_pos gauge\nskylink_aircraft_with_pos {}\n\
         # HELP skylink_messages_total Total messages processed\n# TYPE skylink_messages_total counter\nskylink_messages_total {}\n",
        total, with_pos, msgs);
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain")], body).into_response()
}

// --- clients.json: connected feeders ---

async fn clients_json(State(store): State<Arc<Store>>) -> Response {
    let clients = store.clients.read().clone();
    let body = serde_json::to_string(&clients).unwrap_or("[]".into());
    json_response(format!(r#"{{"now":{:.1},"clients":{}}}"#, now_secs(), body))
}

// --- receivers.json: receiver UUID list ---

async fn receivers_json(State(store): State<Arc<Store>>) -> Response {
    let body = store.receivers_cache.read().clone();
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache")], body).into_response()
}

// --- receiver.json ---

async fn receiver_json() -> Response {
    json_response(r#"{"refresh":1000,"history":0,"readsb":true,"dbServer":true,"haveTraces":false,"globeIndexGrid":3,"globeIndexSpecialTiles":[],"reapi":true,"binCraft":true,"zstd":false,"version":"skylink-core 0.3.0 (Rust)"}"#.into())
}

async fn receiver_pb() -> Response {
    use prost::Message;
    let r = crate::pb::readsb::Receiver {
        version: "skylink-core 0.3.0 (Rust)".into(),
        refresh: 1.0,
        history: 0,
        ..Default::default()
    };
    let body = r.encode_to_vec();
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/x-protobuf"), (header::CACHE_CONTROL, "no-cache")], body).into_response()
}

// --- re-api: unified query with filters ---

async fn re_api(State(store): State<Arc<Store>>, Query(params): Query<HashMap<String, String>>) -> Response {
    let use_zstd = params.contains_key("zstd");
    let bbox = parse_box(&params);
    let circle = parse_circle(&params);

    // Build filter closure
    let find_hex: Option<u32> = params.get("find_hex").and_then(|h| u32::from_str_radix(h, 16).ok());
    let find_cs: Option<String> = params.get("find_callsign").map(|s| s.to_uppercase());
    let filter_squawk: Option<String> = params.get("filter_squawk").cloned();
    let filter_mil = params.contains_key("filter_mil");
    let above_alt: Option<i32> = params.get("above_alt_baro").and_then(|s| s.parse().ok());
    let below_alt: Option<i32> = params.get("below_alt_baro").and_then(|s| s.parse().ok());
    let all_with_pos = params.contains_key("all_with_pos");

    let has_filter = find_hex.is_some() || find_cs.is_some() || filter_squawk.is_some()
        || filter_mil || above_alt.is_some() || below_alt.is_some() || all_with_pos
        || bbox.is_some() || circle.is_some();

    // If no filters, serve from cache
    if !has_filter {
        return serve_format_cached(&store, &params, use_zstd);
    }

    // Filtered: collect matching ICAOs
    let filtered: Vec<(u32, crate::aircraft::Aircraft)> = store.map.iter().filter_map(|entry| {
        let ac = entry.value();
        let icao = *entry.key();

        if let Some(hex) = find_hex { if icao != hex { return None; } }
        if let Some(ref cs) = find_cs {
            match &ac.flight { Some(f) if f.to_uppercase().starts_with(cs) => {}, _ => return None }
        }
        if let Some(ref sq) = filter_squawk {
            match &ac.squawk { Some(s) if s == sq => {}, _ => return None }
        }
        if filter_mil && ac.addr_type != 7 { return None; } // rough mil filter
        if let Some(above) = above_alt {
            match ac.alt_baro { Some(a) if a >= above => {}, _ => return None }
        }
        if let Some(below) = below_alt {
            match ac.alt_baro { Some(a) if a <= below => {}, _ => return None }
        }
        if all_with_pos && ac.lat.is_none() { return None; }
        if let Some((s, n, w, e)) = bbox {
            match (ac.lat, ac.lon) {
                (Some(lat), Some(lon)) if lat >= s && lat <= n && crate::bincraft::lon_in_box(lon, w, e) => {}
                _ => return None,
            }
        }
        if let Some((clat, clon, radius_nm)) = circle {
            match (ac.lat, ac.lon) {
                (Some(lat), Some(lon)) => {
                    let d = haversine_nm(clat, clon, lat, lon);
                    if d > radius_nm { return None; }
                }
                _ => return None,
            }
        }

        Some((icao, ac.clone()))
    }).collect();

    // Encode in requested format
    let format = if params.contains_key("compact") { "compact" }
        else if params.contains_key("binCraft") { "binCraft" }
        else if params.contains_key("pb") { "pb" }
        else { "json" };

    let raw = encode_filtered(&filtered, format);
    serve_binary(raw, use_zstd, match format {
        "pb" => "application/x-protobuf",
        "json" => "application/json",
        _ => "application/octet-stream",
    })
}

fn serve_format_cached(store: &Arc<Store>, params: &HashMap<String, String>, use_zstd: bool) -> Response {
    if params.contains_key("compact") {
        let c = if use_zstd { store.compact_zstd_cache.read().clone() } else { store.compact_cache.read().clone() };
        return serve_cached(c, "application/octet-stream");
    }
    if params.contains_key("binCraft") {
        let c = if use_zstd { store.bincraft_zstd_cache.read().clone() } else { store.bincraft_cache.read().clone() };
        return serve_cached(c, "application/octet-stream");
    }
    if params.contains_key("pb") {
        let c = if use_zstd { store.pb_zstd_cache.read().clone() } else { store.pb_cache.read().clone() };
        return serve_cached(c, "application/x-protobuf");
    }
    let c = if use_zstd { store.json_zstd_cache.read().clone() } else { store.json_cache.read().clone() };
    serve_cached(c, "application/json")
}

fn encode_filtered(aircraft: &[(u32, crate::aircraft::Aircraft)], format: &str) -> Vec<u8> {
    let now_s = now_secs();
    match format {
        "json" => {
            let mut buf = Vec::with_capacity(aircraft.len() * 400);
            buf.extend_from_slice(b"{\"now\":");
            buf.extend_from_slice(format!("{:.1}", now_s).as_bytes());
            buf.extend_from_slice(b",\"aircraft\":[");
            for (i, (_, ac)) in aircraft.iter().enumerate() {
                if i > 0 { buf.push(b','); }
                buf.extend_from_slice(&serde_json::to_vec(ac).unwrap_or_default());
            }
            buf.extend_from_slice(b"]}");
            buf
        }
        "binCraft" => {
            let now_ms = (now_s * 1000.0) as u64;
            let mut buf = Vec::with_capacity(112 + aircraft.len() * 112);
            // Minimal header
            let mut hdr = [0u8; 112];
            hdr[0..4].copy_from_slice(&(now_ms as u32).to_le_bytes());
            hdr[4..8].copy_from_slice(&((now_ms >> 32) as u32).to_le_bytes());
            hdr[8..12].copy_from_slice(&112u32.to_le_bytes());
            hdr[12..16].copy_from_slice(&(aircraft.len() as u32).to_le_bytes());
            hdr[40..44].copy_from_slice(&20250403u32.to_le_bytes());
            buf.extend_from_slice(&hdr);
            for (icao, ac) in aircraft {
                buf.extend_from_slice(&crate::bincraft::encode_aircraft_pub(*icao, ac, now_s));
            }
            buf
        }
        "compact" => {
            let now_ms = (now_s * 1000.0) as u64;
            let mut buf = Vec::with_capacity(12 + aircraft.len() * 60);
            buf.extend_from_slice(&now_ms.to_le_bytes());
            buf.extend_from_slice(&(aircraft.len() as u32).to_le_bytes());
            for (icao, ac) in aircraft {
                crate::compact::encode_compact_pub(*icao, ac, now_s, &mut buf);
            }
            buf
        }
        "pb" => {
            crate::pb::build_from_list(aircraft, now_s)
        }
        _ => vec![],
    }
}

// --- Traces: flight path history ---

async fn trace_full(State(store): State<Arc<Store>>, Path(hex): Path<String>) -> Response {
    let icao = match u32::from_str_radix(&hex, 16) {
        Ok(v) => v, Err(_) => return (StatusCode::NOT_FOUND, "invalid hex").into_response(),
    };
    let entry = match store.map.get(&icao) {
        Some(e) => e, None => return (StatusCode::NOT_FOUND, "not found").into_response(),
    };
    let ac = entry.value();
    let now = now_secs();

    // readsb trace format: {"icao":"hex","timestamp":first_ts,"trace":[[ts,lat,lon,alt_baro,gs,track,flags,...],...]}
    let mut buf = Vec::with_capacity(ac.trace.len() * 80 + 200);
    buf.extend_from_slice(b"{\"icao\":\"");
    buf.extend_from_slice(ac.hex.as_bytes());
    buf.extend_from_slice(b"\",\"noRegData\":true");
    if let Some(ref f) = ac.flight {
        buf.extend_from_slice(b",\"callsign\":\"");
        buf.extend_from_slice(f.as_bytes());
        buf.push(b'"');
    }
    if !ac.trace.is_empty() {
        buf.extend_from_slice(b",\"timestamp\":");
        buf.extend_from_slice(format!("{:.3}", ac.trace[0].ts).as_bytes());
    }
    buf.extend_from_slice(b",\"trace\":[");
    for (i, p) in ac.trace.iter().enumerate() {
        if i > 0 { buf.push(b','); }
        // [relative_ts, lat, lon, alt_baro, gs, track, flags, vert_rate, ias, alt_geom]
        let rel = if !ac.trace.is_empty() { p.ts - ac.trace[0].ts } else { 0.0 };
        buf.push(b'[');
        buf.extend_from_slice(format!("{:.1},{:.6},{:.6},", rel, p.lat, p.lon).as_bytes());
        match p.alt_baro { Some(a) => buf.extend_from_slice(a.to_string().as_bytes()), None => buf.extend_from_slice(b"null") };
        buf.push(b',');
        match p.gs { Some(v) => buf.extend_from_slice(format!("{:.1}", v).as_bytes()), None => buf.extend_from_slice(b"null") };
        buf.push(b',');
        match p.track { Some(v) => buf.extend_from_slice(format!("{:.1}", v).as_bytes()), None => buf.extend_from_slice(b"null") };
        buf.extend_from_slice(b",0,"); // flags
        match p.baro_rate { Some(v) => buf.extend_from_slice(v.to_string().as_bytes()), None => buf.extend_from_slice(b"null") };
        buf.push(b',');
        match p.ias { Some(v) => buf.extend_from_slice(v.to_string().as_bytes()), None => buf.extend_from_slice(b"null") };
        buf.push(b',');
        match p.alt_geom { Some(v) => buf.extend_from_slice(v.to_string().as_bytes()), None => buf.extend_from_slice(b"null") };
        buf.push(b']');
    }
    buf.extend_from_slice(b"]}");
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], buf).into_response()
}

// Recent trace: last 2 minutes
async fn trace_recent(State(store): State<Arc<Store>>, Path(hex): Path<String>) -> Response {
    let icao = match u32::from_str_radix(&hex, 16) {
        Ok(v) => v, Err(_) => return (StatusCode::NOT_FOUND, "invalid hex").into_response(),
    };
    let entry = match store.map.get(&icao) {
        Some(e) => e, None => return (StatusCode::NOT_FOUND, "not found").into_response(),
    };
    let ac = entry.value();
    let now = now_secs();
    let cutoff = now - 120.0;

    let recent: Vec<&crate::aircraft::TracePoint> = ac.trace.iter().filter(|p| p.ts >= cutoff).collect();

    let mut buf = Vec::with_capacity(recent.len() * 80 + 200);
    buf.extend_from_slice(b"{\"icao\":\"");
    buf.extend_from_slice(ac.hex.as_bytes());
    buf.extend_from_slice(b"\"");
    if !recent.is_empty() {
        buf.extend_from_slice(b",\"timestamp\":");
        buf.extend_from_slice(format!("{:.3}", recent[0].ts).as_bytes());
    }
    buf.extend_from_slice(b",\"trace\":[");
    for (i, p) in recent.iter().enumerate() {
        if i > 0 { buf.push(b','); }
        let rel = if !recent.is_empty() { p.ts - recent[0].ts } else { 0.0 };
        buf.push(b'[');
        buf.extend_from_slice(format!("{:.1},{:.6},{:.6},", rel, p.lat, p.lon).as_bytes());
        match p.alt_baro { Some(a) => buf.extend_from_slice(a.to_string().as_bytes()), None => buf.extend_from_slice(b"null") };
        buf.push(b',');
        match p.gs { Some(v) => buf.extend_from_slice(format!("{:.1}", v).as_bytes()), None => buf.extend_from_slice(b"null") };
        buf.push(b',');
        match p.track { Some(v) => buf.extend_from_slice(format!("{:.1}", v).as_bytes()), None => buf.extend_from_slice(b"null") };
        buf.extend_from_slice(b",0,null,null,null]");
    }
    buf.extend_from_slice(b"]}");
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], buf).into_response()
}

// --- stats (simple) ---

async fn stats(State(store): State<Arc<Store>>) -> Response {
    let total = store.map.len();
    let with_pos = store.map.iter().filter(|e| e.value().lat.is_some()).count();
    let msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed);
    json_response(format!(r#"{{"aircraft_total":{},"aircraft_with_pos":{},"messages_total":{}}}"#, total, with_pos, msgs))
}

// --- Helpers ---

fn json_response(body: String) -> Response {
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-cache")], body).into_response()
}

fn serve_binary(raw: Vec<u8>, use_zstd: bool, ct: &'static str) -> Response {
    let body = if use_zstd { zstd::encode_all(raw.as_slice(), 3).unwrap_or(raw) } else { raw };
    (StatusCode::OK, [(header::CONTENT_TYPE, ct), (header::CACHE_CONTROL, "no-cache"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], body).into_response()
}

fn now_secs() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()
}

fn parse_box(p: &HashMap<String, String>) -> Option<(f64, f64, f64, f64)> {
    p.get("box").and_then(|b| {
        let v: Vec<f64> = b.split(',').filter_map(|s| s.parse().ok()).collect();
        if v.len() == 4 { Some((v[0], v[1], v[2], v[3])) } else { None }
    })
}

fn parse_circle(p: &HashMap<String, String>) -> Option<(f64, f64, f64)> {
    p.get("circle").and_then(|c| {
        let v: Vec<f64> = c.split(',').filter_map(|s| s.parse().ok()).collect();
        if v.len() == 3 { Some((v[0], v[1], v[2])) } else { None }
    })
}

fn haversine_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (r1, r2) = (lat1.to_radians(), lat2.to_radians());
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + r1.cos() * r2.cos() * (dlon / 2.0).sin().powi(2);
    3440.065 * 2.0 * a.sqrt().asin() // nm
}

// --- Router ---

pub async fn serve(store: Arc<Store>, port: u16) {
    let app = Router::new()
        // Data endpoints
        .route("/data/aircraft.json", get(aircraft_json))
        .route("/data/aircraft.binCraft", get(aircraft_bincraft))
        .route("/data/aircraft.pb", get(aircraft_pb))
        .route("/data/aircraft.pb.zst", get(aircraft_pb_zstd))
        .route("/data/aircraft.compact", get(aircraft_compact))
        .route("/data/aircraft_recent.json", get(aircraft_recent))
        .route("/data/receiver.json", get(receiver_json))
        .route("/data/receiver.pb", get(receiver_pb))
        .route("/data/status.json", get(status_json))
        .route("/data/status.prom", get(status_prom))
        .route("/data/clients.json", get(clients_json))
        .route("/data/receivers.json", get(receivers_json))
        // Traces
        .route("/data/traces/{hex}/trace_full.json", get(trace_full))
        .route("/data/traces/{hex}/trace_recent.json", get(trace_recent))
        // Query API
        .route("/re-api/", get(re_api))
        // WebSocket
        .route("/ws", get(crate::ws::ws_handler))
        // Stats
        .route("/stats", get(stats))
        .layer(CorsLayer::permissive())
        .with_state(store);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("failed to bind API port");
    info!("API on :{}", port);
    axum::serve(listener, app).await.unwrap();
}
