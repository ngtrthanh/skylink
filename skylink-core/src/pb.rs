/// Protobuf output — wire-compatible with readsb-protobuf

pub mod readsb {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

use crate::aircraft::Store;
use prost::Message;
use std::sync::Arc;

pub fn build_aircraft_pb(store: &Arc<Store>) -> Vec<u8> {
    build_pb_inner(store, None)
}

pub fn build_filtered(store: &Arc<Store>, south: f64, north: f64, west: f64, east: f64) -> Vec<u8> {
    build_pb_inner(store, Some((south, north, west, east)))
}

fn make_ac_meta(icao: u32, ac: &crate::aircraft::Aircraft, seen_ms: u64, now_s: f64) -> readsb::AircraftMeta {
    readsb::AircraftMeta {
        addr: icao,
        flight: ac.flight.clone().unwrap_or_default(),
        squawk: ac.squawk.as_ref().and_then(|s| u32::from_str_radix(s, 8).ok()).unwrap_or(0),
        category: ac.category.as_ref().and_then(|c| u32::from_str_radix(c, 16).ok()).unwrap_or(0),
        alt_baro: ac.alt_baro.unwrap_or(0),
        alt_geom: ac.alt_geom.unwrap_or(0),
        lat: ac.lat.unwrap_or(0.0), lon: ac.lon.unwrap_or(0.0),
        messages: ac.messages, seen: seen_ms,
        rssi: ac.rssi.unwrap_or(-50.0) as f32,
        gs: ac.gs.map(|v| v as u32).unwrap_or(0),
        track: ac.track.map(|v| v as i32).unwrap_or(0),
        baro_rate: ac.baro_rate.unwrap_or(0), geom_rate: ac.geom_rate.unwrap_or(0),
        ias: ac.ias.unwrap_or(0) as u32, tas: ac.tas.unwrap_or(0) as u32,
        mach: ac.mach.unwrap_or(0.0) as f32,
        mag_heading: ac.mag_heading.map(|v| v as i32).unwrap_or(0),
        true_heading: ac.true_heading.map(|v| v as i32).unwrap_or(0),
        roll: ac.roll.map(|v| v as f32).unwrap_or(0.0),
        track_rate: ac.track_rate.map(|v| v as f32).unwrap_or(0.0),
        nav_altitude_mcp: ac.nav_altitude_mcp.map(|v| v as i32).unwrap_or(0),
        nav_altitude_fms: ac.nav_altitude_fms.map(|v| v as i32).unwrap_or(0),
        nav_qnh: ac.nav_qnh.map(|v| v as f32).unwrap_or(0.0),
        nav_heading: ac.nav_heading.map(|v| v as i32).unwrap_or(0),
        nic: ac.nic.unwrap_or(0) as u32, nac_p: ac.nac_p.unwrap_or(0) as u32,
        nac_v: ac.nac_v.unwrap_or(0) as u32, sil: ac.sil.unwrap_or(0) as u32,
        gva: ac.gva.unwrap_or(0) as u32, sda: ac.sda.unwrap_or(0) as u32,
        nic_baro: ac.nic_baro.unwrap_or(0) as u32,
        version: ac.adsb_version.map(|v| v as i32).unwrap_or(-1),
        seen_pos: ac.seen_pos.map(|_| (now_s - ac.last_update) as u32).unwrap_or(0),
        air_ground: if ac.lat.is_some() { 2 } else { 0 },
        emergency: ac.emergency.unwrap_or(0) as i32,
        addr_type: ac.addr_type as i32,
        ..Default::default()
    }
}

pub fn build_from_list(aircraft: &[(u32, crate::aircraft::Aircraft)], now_s: f64) -> Vec<u8> {
    let now_ms = (now_s * 1000.0) as u64;
    let ac_list: Vec<readsb::AircraftMeta> = aircraft.iter().map(|(icao, ac)| {
        let seen_ms = ((now_s - ac.last_update) * 1000.0) as u64;
        make_ac_meta(*icao, ac, seen_ms, now_s)
    }).collect();
    let update = readsb::AircraftsUpdate { now: now_ms / 1000, messages: 0, aircraft: ac_list, history: vec![] };
    use prost::Message;
    update.encode_to_vec()
}

fn build_pb_inner(store: &Arc<Store>, bbox: Option<(f64, f64, f64, f64)>) -> Vec<u8> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let total_msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed);
    let now_s = now_ms as f64 / 1000.0;

    let aircraft: Vec<readsb::AircraftMeta> = store.map.iter().filter_map(|entry| {
        let ac = entry.value();
        if let Some((s, n, w, e)) = bbox {
            match (ac.lat, ac.lon) {
                (Some(lat), Some(lon)) if lat >= s && lat <= n && crate::bincraft::lon_in_box(lon, w, e) => {},
                _ => return None,
            }
        }
        let icao = *entry.key();
        let seen_ms = ((now_s - ac.last_update) * 1000.0) as u64;

        Some(make_ac_meta(icao, ac, seen_ms, now_s))
    }).collect();

    let update = readsb::AircraftsUpdate {
        now: now_ms / 1000,
        messages: total_msgs,
        aircraft,
        history: vec![],
    };

    update.encode_to_vec()
}
