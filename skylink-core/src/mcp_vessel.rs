/// MCP vessel tool endpoints
use std::sync::Arc;
use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::json;
use crate::ais::vessel::VesselStore;

#[derive(Deserialize)]
pub struct VesselSearchParams {
    pub name: Option<String>,
    pub mmsi: Option<u32>,
    pub shiptype: Option<u8>,
    pub country: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct VesselAreaParams {
    pub south: f64, pub north: f64, pub west: f64, pub east: f64,
    pub limit: Option<usize>,
}

fn now() -> f64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64() }

fn vessel_json(v: &crate::ais::vessel::Vessel, t: f64) -> serde_json::Value {
    json!({
        "mmsi": v.mmsi, "shipname": v.shipname, "callsign": v.callsign,
        "lat": v.lat, "lon": v.lon, "speed": v.speed, "cog": v.cog, "heading": v.heading,
        "shiptype": v.shiptype, "type_class": v.type_class(),
        "shipclass": v.shipclass, "class_name": v.class_name(),
        "country": v.country, "destination": v.destination,
        "imo": v.imo, "status": v.status,
        "seen": format!("{:.0}s", t - v.last_signal),
    })
}

pub async fn vessel_search(State(store): State<Arc<VesselStore>>, Json(p): Json<VesselSearchParams>) -> Json<serde_json::Value> {
    let limit = p.limit.unwrap_or(20);
    let t = now();
    let mut results = Vec::new();
    for entry in store.map.iter() {
        if results.len() >= limit { break; }
        let v = entry.value();
        if t - v.last_signal > 600.0 { continue; }
        if let Some(mmsi) = p.mmsi { if v.mmsi != mmsi { continue; } }
        if let Some(ref name) = p.name {
            if !v.shipname.to_uppercase().contains(&name.to_uppercase()) { continue; }
        }
        if let Some(st) = p.shiptype { if v.shiptype != st { continue; } }
        if let Some(ref c) = p.country { if v.country != *c { continue; } }
        if p.name.is_none() && p.mmsi.is_none() && p.shiptype.is_none() && p.country.is_none() && v.lat.is_none() { continue; }
        results.push(vessel_json(v, t));
    }
    Json(json!({"count": results.len(), "vessels": results}))
}

pub async fn vessel_area(State(store): State<Arc<VesselStore>>, Json(p): Json<VesselAreaParams>) -> Json<serde_json::Value> {
    let limit = p.limit.unwrap_or(50);
    let t = now();
    let mut results = Vec::new();
    for entry in store.map.iter() {
        if results.len() >= limit { break; }
        let v = entry.value();
        if t - v.last_signal > 600.0 { continue; }
        if let (Some(lat), Some(lon)) = (v.lat, v.lon) {
            let lat = lat as f64; let lon = lon as f64;
            if lat >= p.south && lat <= p.north {
                let lon_ok = if p.west <= p.east { lon >= p.west && lon <= p.east } else { lon >= p.west || lon <= p.east };
                if lon_ok { results.push(vessel_json(v, t)); }
            }
        }
    }
    Json(json!({"count": results.len(), "vessels": results}))
}
