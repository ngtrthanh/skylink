/// Protobuf output — wire-compatible with readsb-protobuf

pub mod readsb {
    include!(concat!(env!("OUT_DIR"), "/_.rs"));
}

use crate::aircraft::Store;
use prost::Message;
use std::sync::Arc;

pub fn build_aircraft_pb(store: &Arc<Store>) -> Vec<u8> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let total_msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed);
    let now_s = now_ms as f64 / 1000.0;

    let aircraft: Vec<readsb::AircraftMeta> = store.map.iter().map(|entry| {
        let ac = entry.value();
        let icao = *entry.key();
        let seen_ms = ((now_s - ac.last_update) * 1000.0) as u64;

        readsb::AircraftMeta {
            addr: icao,
            flight: ac.flight.clone().unwrap_or_default(),
            squawk: ac.squawk.as_ref().and_then(|s| u32::from_str_radix(s, 8).ok()).unwrap_or(0),
            category: ac.category.as_ref().and_then(|c| u32::from_str_radix(c, 16).ok()).unwrap_or(0),
            alt_baro: ac.alt_baro.unwrap_or(0),
            lat: ac.lat.unwrap_or(0.0),
            lon: ac.lon.unwrap_or(0.0),
            messages: ac.messages,
            seen: seen_ms,
            rssi: ac.rssi.unwrap_or(-50.0) as f32,
            gs: ac.gs.map(|v| v as u32).unwrap_or(0),
            track: ac.track.map(|v| v as i32).unwrap_or(0),
            alt_geom: ac.alt_geom.unwrap_or(0),
            baro_rate: ac.baro_rate.unwrap_or(0),
            seen_pos: ac.seen_pos.map(|_| ((now_s - ac.last_update) as u32)).unwrap_or(0),
            air_ground: if ac.lat.is_some() { 2 } else { 0 }, // AG_AIRBORNE or AG_INVALID
            ..Default::default()
        }
    }).collect();

    let update = readsb::AircraftsUpdate {
        now: now_ms / 1000,
        messages: total_msgs,
        aircraft,
        history: vec![],
    };

    update.encode_to_vec()
}
