/// MCP-compatible tool endpoints served as regular HTTP
/// Clients can discover tools via /.well-known/mcp.json
/// Each tool is a POST endpoint that accepts JSON params and returns JSON

use std::sync::Arc;
use axum::{extract::State, response::{IntoResponse, Response}, http::{header, StatusCode}, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::aircraft::Store;

#[derive(Deserialize)]
pub struct SearchParams {
    pub callsign: Option<String>,
    pub hex: Option<String>,
    pub squawk: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct HexParam { pub hex: String }

#[derive(Deserialize)]
pub struct AreaParams {
    pub south: f64, pub north: f64, pub west: f64, pub east: f64,
    pub limit: Option<usize>,
}

fn now() -> f64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64() }

fn ac_json(ac: &crate::aircraft::Aircraft, t: f64) -> serde_json::Value {
    json!({"hex":ac.hex,"callsign":ac.flight,"lat":ac.lat,"lon":ac.lon,
           "alt_baro":ac.alt_baro,"gs":ac.gs,"track":ac.track,
           "squawk":ac.squawk,"seen":format!("{:.0}s",t-ac.last_update),"messages":ac.messages})
}

pub async fn manifest() -> Response {
    let body = json!({
        "schema_version": "1.0",
        "name": "skylink-core",
        "description": "Live ADS-B aircraft tracker. Query aircraft worldwide by callsign, ICAO hex, squawk, or geographic area.",
        "tools": [
            {"name":"search_aircraft","description":"Search aircraft by callsign prefix, ICAO hex, or squawk","endpoint":"/mcp/search","method":"POST"},
            {"name":"get_trace","description":"Get flight path history for an aircraft","endpoint":"/mcp/trace","method":"POST"},
            {"name":"list_area","description":"List aircraft in a bounding box","endpoint":"/mcp/area","method":"POST"},
            {"name":"get_stats","description":"Get aggregator statistics","endpoint":"/mcp/stats","method":"GET"},
        ]
    });
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], Json(body)).into_response()
}

pub async fn search(State(store): State<Arc<Store>>, Json(p): Json<SearchParams>) -> Json<serde_json::Value> {
    let limit = p.limit.unwrap_or(20);
    let t = now();
    let hex_filter = p.hex.and_then(|h| u32::from_str_radix(&h, 16).ok());
    let mut results = Vec::new();
    for entry in store.map.iter() {
        if results.len() >= limit { break; }
        let ac = entry.value();
        let icao = *entry.key();
        if let Some(h) = hex_filter { if icao != h { continue; } }
        if let Some(ref cs) = p.callsign {
            match &ac.flight { Some(f) if f.to_uppercase().starts_with(&cs.to_uppercase()) => {}, _ => continue }
        }
        if let Some(ref sq) = p.squawk {
            match &ac.squawk { Some(s) if s == sq => {}, _ => continue }
        }
        if p.callsign.is_none() && hex_filter.is_none() && p.squawk.is_none() && ac.lat.is_none() { continue; }
        results.push(ac_json(ac, t));
    }
    Json(json!({"count": results.len(), "aircraft": results}))
}

pub async fn trace(State(store): State<Arc<Store>>, Json(p): Json<HexParam>) -> Json<serde_json::Value> {
    let icao = match u32::from_str_radix(&p.hex, 16) { Ok(v) => v, Err(_) => return Json(json!({"error":"invalid hex"})) };
    match store.map.get(&icao) {
        Some(e) => {
            let ac = e.value();
            let pts: Vec<_> = ac.trace.iter().map(|p| json!({"ts":p.ts,"lat":p.lat,"lon":p.lon,"alt":p.alt_baro,"gs":p.gs})).collect();
            Json(json!({"hex":ac.hex,"callsign":ac.flight,"points":pts.len(),"trace":pts}))
        }
        None => Json(json!({"error":"not found"}))
    }
}

pub async fn area(State(store): State<Arc<Store>>, Json(p): Json<AreaParams>) -> Json<serde_json::Value> {
    let limit = p.limit.unwrap_or(50);
    let t = now();
    let mut results = Vec::new();
    for entry in store.map.iter() {
        if results.len() >= limit { break; }
        let ac = entry.value();
        if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
            if lat >= p.south && lat <= p.north && crate::bincraft::lon_in_box(lon, p.west, p.east) {
                results.push(ac_json(ac, t));
            }
        }
    }
    Json(json!({"count": results.len(), "aircraft": results}))
}

pub async fn stats(State(store): State<Arc<Store>>) -> Json<serde_json::Value> {
    let total = store.map.len();
    let with_pos = store.map.iter().filter(|e| e.value().lat.is_some()).count();
    let msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed);
    Json(json!({"aircraft_total":total,"aircraft_with_position":with_pos,"messages_total":msgs,"uptime_seconds":now()-store.start_time}))
}
