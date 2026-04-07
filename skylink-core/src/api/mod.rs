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
async fn aircraft_bincraft_zst(State(s): State<Arc<Store>>) -> Response { serve_cached(s.bincraft_zstd_cache.read().clone(), "application/octet-stream") }
async fn aircraft_json_zst(State(s): State<Arc<Store>>) -> Response { serve_cached(s.json_zstd_cache.read().clone(), "application/json") }
async fn aircraft_pb(State(s): State<Arc<Store>>) -> Response { serve_cached(s.pb_cache.read().clone(), "application/x-protobuf") }
async fn aircraft_pb_zstd(State(s): State<Arc<Store>>) -> Response { serve_cached(s.pb_zstd_cache.read().clone(), "application/x-protobuf") }
async fn aircraft_compact(State(s): State<Arc<Store>>) -> Response { serve_cached(s.compact_zstd_cache.read().clone(), "application/octet-stream") }
async fn aircraft_geojson(State(s): State<Arc<Store>>) -> Response { serve_cached(s.geojson_cache.read().clone(), "application/geo+json") }
async fn aircraft_geojson_zst(State(s): State<Arc<Store>>) -> Response { serve_cached(s.geojson_zstd_cache.read().clone(), "application/geo+json") }

async fn sprite_json() -> Response {
    let body = r#"{"aircraft":{"width":32,"height":32,"x":0,"y":0,"sdf":true,"pixelRatio":1}}"#;
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}
async fn sprite_png() -> Response {
    let png = include_bytes!("../../aircraft-icon.png");
    (StatusCode::OK, [(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "public, max-age=86400")], png.as_slice()).into_response()
}

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
    json_response(r#"{"refresh":1000,"history":0,"readsb":true,"dbServer":true,"haveTraces":false,"globeIndexGrid":3,"globeIndexSpecialTiles":[],"reapi":true,"binCraft":true,"zstd":true,"version":"skylink-core 0.3.0 (Rust)"}"#.into())
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

    // Fast path: bbox-only + binCraft → filter from pre-built cache (no re-encoding)
    if bbox.is_some() && !has_filter_except_bbox(&find_hex, &find_cs, &filter_squawk, filter_mil, above_alt, below_alt, all_with_pos, circle) {
        if params.contains_key("binCraft") {
            let (s, n, w, e) = bbox.unwrap();
            let cache = store.bincraft_cache.read().clone();
            let raw = crate::bincraft::build_filtered_from_cache(&cache, s, n, w, e);
            return serve_binary(raw, use_zstd, "application/octet-stream");
        }
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
        else if params.contains_key("geojson") { "geojson" }
        else { "json" };

    let raw = encode_filtered(&filtered, format);
    serve_binary(raw, use_zstd, match format {
        "pb" => "application/x-protobuf",
        "json" => "application/json",
        "geojson" => "application/geo+json",
        _ => "application/octet-stream",
    })
}

fn serve_format_cached(store: &Arc<Store>, params: &HashMap<String, String>, use_zstd: bool) -> Response {
    if params.contains_key("geojson") {
        let c = if use_zstd { store.geojson_zstd_cache.read().clone() } else { store.geojson_cache.read().clone() };
        return serve_cached(c, "application/geo+json");
    }

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
        "geojson" => {
            let mut buf = Vec::with_capacity(aircraft.len() * 300);
            buf.extend_from_slice(b"{\"type\":\"FeatureCollection\",\"features\":[");
            for (i, (_, ac)) in aircraft.iter().enumerate() {
                if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
                    if i > 0 { buf.push(b','); }
                    buf.extend_from_slice(b"{\"type\":\"Feature\",\"geometry\":{\"type\":\"Point\",\"coordinates\":[");
                    buf.extend_from_slice(format!("{:.6},{:.6}", lon, lat).as_bytes());
                    buf.extend_from_slice(b"]},\"properties\":");
                    buf.extend_from_slice(&serde_json::to_vec(ac).unwrap_or_default());
                    buf.push(b'}');
                }
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

async fn globe_fallback(State(s): State<Arc<Store>>, axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    if path.ends_with(".binCraft.zst") || path.ends_with(".binCraft.zst") {
        return serve_cached(s.bincraft_zstd_cache.read().clone(), "application/octet-stream");
    }
    if path.ends_with(".binCraft") {
        return serve_cached(s.bincraft_cache.read().clone(), "application/octet-stream");
    }
    (StatusCode::NOT_FOUND, "not found").into_response()
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
    let body = if use_zstd { zstd_with_size(&raw) } else { raw };
    (StatusCode::OK, [(header::CONTENT_TYPE, ct), (header::CACHE_CONTROL, "no-cache"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], body).into_response()
}

fn zstd_with_size(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut out = Vec::with_capacity(data.len() / 2);
    let mut enc = zstd::Encoder::new(&mut out, 3).unwrap();
    enc.set_pledged_src_size(Some(data.len() as u64)).unwrap();
    enc.write_all(data).unwrap();
    enc.finish().unwrap();
    out
}

fn now_secs() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()
}

fn has_filter_except_bbox(
    hex: &Option<u32>, cs: &Option<String>, sq: &Option<String>,
    mil: bool, above: Option<i32>, below: Option<i32>, awp: bool, circle: Option<(f64,f64,f64)>,
) -> bool {
    hex.is_some() || cs.is_some() || sq.is_some() || mil || above.is_some() || below.is_some() || awp || circle.is_some()
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

pub async fn serve(aircraft_store: Option<Arc<Store>>, vessel_store: Option<Arc<crate::ais::vessel::VesselStore>>, cfg: crate::config::Config) {
    let port = cfg.api.port;

    // Build aircraft routes (if enabled)
    let mut app = Router::new();

    if let Some(ref store) = aircraft_store {
        let ac_routes = Router::new()
            .route("/data/aircraft.json", get(aircraft_json))
            .route("/data/aircraft.binCraft", get(aircraft_bincraft))
            .route("/data/aircraft.binCraft.zst", get(aircraft_bincraft_zst))
            .route("/data/aircraft.json.zst", get(aircraft_json_zst))
            .route("/data/aircraft.pb", get(aircraft_pb))
            .route("/data/aircraft.pb.zst", get(aircraft_pb_zstd))
            .route("/data/aircraft.compact", get(aircraft_compact))
            .route("/data/aircraft.geojson", get(aircraft_geojson))
            .route("/data/aircraft.geojson.zst", get(aircraft_geojson_zst))
            .route("/sprite.json", get(sprite_json))
            .route("/sprite.png", get(sprite_png))
            .route("/data/aircraft_recent.json", get(aircraft_recent))
            .route("/data/receiver.json", get(receiver_json))
            .route("/data/receiver.pb", get(receiver_pb))
            .route("/data/status.json", get(status_json))
            .route("/data/status.prom", get(status_prom))
            .route("/data/clients.json", get(clients_json))
            .route("/data/receivers.json", get(receivers_json))
            .route("/data/traces/{hex}/trace_full.json", get(trace_full))
            .route("/data/traces/{hex}/trace_recent.json", get(trace_recent))
            .route("/re-api/", get(re_api))
            .route("/ws", get(crate::ws::ws_handler))
            .route("/.well-known/mcp.json", get(crate::mcp::manifest))
            .route("/mcp/search", axum::routing::post(crate::mcp::search))
            .route("/mcp/trace", axum::routing::post(crate::mcp::trace))
            .route("/mcp/area", axum::routing::post(crate::mcp::area))
            .route("/mcp/stats", get(crate::mcp::stats))
            .route("/data/{*path}", get(globe_fallback))
            .with_state(store.clone());
        app = app.merge(ac_routes);
    }

    // Build vessel routes (if enabled)
    if let Some(ref store) = vessel_store {
        let vs_routes = Router::new()
            .route("/api/vessels.json", get(vessels_json))
            .route("/api/vessels.geojson", get(vessels_geojson))
            .route("/api/vessel", get(vessel_detail))
            .route("/api/path.json", get(vessel_path_json))
            .route("/api/path.geojson", get(vessel_path_geojson))
            .route("/api/allpath.geojson", get(vessel_allpath_geojson))
            .route("/api/ais_stats.json", get(ais_stats_json))
            .route("/ws/ais", get(crate::ws_ais::ws_handler))
            .route("/mcp/vessel_search", axum::routing::post(crate::mcp_vessel::vessel_search))
            .route("/mcp/vessel_area", axum::routing::post(crate::mcp_vessel::vessel_area))
            .with_state(store.clone());
        app = app.merge(vs_routes);
    }

    // Unified WS (both aircraft + vessels in one connection)
    if aircraft_store.is_some() || vessel_store.is_some() {
        let unified = Arc::new(crate::ws_unified::UnifiedState {
            aircraft: aircraft_store.clone(),
            vessels: vessel_store.clone(),
        });
        let unified_route = Router::new()
            .route("/ws/unified", get(crate::ws_unified::ws_handler))
            .with_state(unified);
        app = app.merge(unified_route);
    }

    // Stats endpoint (works with whatever is enabled)
    let ac = aircraft_store.clone();
    let vs = vessel_store.clone();
    app = app.route("/stats", get(move || {
        let ac = ac.clone();
        let vs = vs.clone();
        async move { combined_stats(ac, vs) }
    }));

    app = app.layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("failed to bind API port");
    info!("API on :{}", port);
    axum::serve(listener, app).await.unwrap();
}

// --- Vessel handlers ---

async fn vessels_json(State(store): State<Arc<crate::ais::vessel::VesselStore>>) -> Response {
    serve_cached(store.json_cache.read().clone(), "application/json")
}

async fn vessels_geojson(State(store): State<Arc<crate::ais::vessel::VesselStore>>, Query(params): Query<HashMap<String, String>>) -> Response {
    if let Some(bbox) = parse_box(&params) {
        let data = store.build_geojson_filtered(bbox.0, bbox.1, bbox.2, bbox.3);
        return (StatusCode::OK, [(header::CONTENT_TYPE, "application/geo+json"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], data).into_response();
    }
    serve_cached(store.geojson_cache.read().clone(), "application/geo+json")
}

async fn vessel_detail(State(store): State<Arc<crate::ais::vessel::VesselStore>>, Query(params): Query<HashMap<String, String>>) -> Response {
    let mmsi: u32 = match params.get("mmsi").and_then(|v| v.parse().ok()) {
        Some(m) => m,
        None => return json_response("{\"error\":\"missing mmsi\"}".into()),
    };
    match store.map.get(&mmsi) {
        Some(v) => {
            let mut out = String::new();
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
            crate::ais::vessel::vessel_json_pub(v.value(), &mut out, now);
            json_response(out)
        }
        None => json_response("{\"error\":\"not found\"}".into()),
    }
}

async fn vessel_path_json(State(store): State<Arc<crate::ais::vessel::VesselStore>>, Query(params): Query<HashMap<String, String>>) -> Response {
    let mmsi: u32 = match params.get("mmsi").and_then(|v| v.parse().ok()) {
        Some(m) => m,
        None => return json_response("{\"error\":\"missing mmsi\"}".into()),
    };
    match store.get_path_json(mmsi) {
        Some(j) => json_response(j),
        None => json_response("{\"error\":\"not found\"}".into()),
    }
}

async fn vessel_path_geojson(State(store): State<Arc<crate::ais::vessel::VesselStore>>, Query(params): Query<HashMap<String, String>>) -> Response {
    let mmsi: u32 = match params.get("mmsi").and_then(|v| v.parse().ok()) {
        Some(m) => m,
        None => return json_response("{\"error\":\"missing mmsi\"}".into()),
    };
    match store.get_path_geojson(mmsi) {
        Some(j) => (StatusCode::OK, [(header::CONTENT_TYPE, "application/geo+json")], j).into_response(),
        None => json_response("{\"error\":\"not found\"}".into()),
    }
}

async fn vessel_allpath_geojson(State(store): State<Arc<crate::ais::vessel::VesselStore>>) -> Response {
    let data = store.get_all_paths_geojson();
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/geo+json"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], data).into_response()
}

async fn ais_stats_json(State(store): State<Arc<crate::ais::vessel::VesselStore>>) -> Response {
    json_response(store.stats.to_json())
}

fn combined_stats(ac: Option<Arc<Store>>, vs: Option<Arc<crate::ais::vessel::VesselStore>>) -> Response {
    let mut out = String::from("{");
    let mut first = true;
    if let Some(s) = ac {
        let total = s.map.len();
        let with_pos = s.map.iter().filter(|e| e.value().lat.is_some()).count();
        let msgs = s.messages_total.load(std::sync::atomic::Ordering::Relaxed);
        out.push_str(&format!("\"aircraft_total\":{total},\"aircraft_with_pos\":{with_pos},\"aircraft_messages\":{msgs}"));
        first = false;
    }
    if let Some(s) = vs {
        if !first { out.push(','); }
        let total = s.map.len();
        let with_pos = s.map.iter().filter(|e| e.value().lat.is_some()).count();
        let msgs = s.messages_total.load(std::sync::atomic::Ordering::Relaxed);
        out.push_str(&format!("\"vessel_total\":{total},\"vessel_with_pos\":{with_pos},\"vessel_messages\":{msgs}"));
    }
    out.push('}');
    json_response(out)
}
