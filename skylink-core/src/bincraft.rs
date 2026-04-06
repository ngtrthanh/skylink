/// binCraft binary format — wire-compatible with readsb
/// Struct: 112 bytes per aircraft, little-endian, packed

use crate::aircraft::{Aircraft, Store};
use std::sync::Arc;

const STRIDE: u32 = 112;
const VERSION: u32 = 20250403;

fn encode_aircraft(icao: u32, ac: &Aircraft, now_s: f64) -> [u8; 112] {
    let mut rec = [0u8; 112];
    let seen_10 = ((now_s - ac.last_update) * 10.0) as i32;

    write_u32(&mut rec, 0, icao);
    write_i32(&mut rec, 4, seen_10);

    if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
        write_i32(&mut rec, 8, (lon * 1e6) as i32);
        write_i32(&mut rec, 12, (lat * 1e6) as i32);
    }

    if let Some(vr) = ac.baro_rate { write_i16(&mut rec, 16, (vr as f64 / 8.0) as i16); }
    if let Some(alt) = ac.alt_baro { write_i16(&mut rec, 20, (alt as f64 / 25.0) as i16); }
    if let Some(alt) = ac.alt_geom { write_i16(&mut rec, 22, (alt as f64 / 25.0) as i16); }

    if let Some(ref sq) = ac.squawk {
        if let Ok(v) = u16::from_str_radix(sq, 16) { write_u16(&mut rec, 32, v); }
    }

    if let Some(gs) = ac.gs { write_i16(&mut rec, 34, (gs * 10.0) as i16); }
    if let Some(trk) = ac.track { write_i16(&mut rec, 40, (trk * 90.0) as i16); }

    write_u16(&mut rec, 62, ac.messages.min(65535) as u16);

    if let Some(ref cat) = ac.category {
        if let Ok(v) = u8::from_str_radix(cat, 16) { rec[64] = v; }
    }

    // u8[67] high nibble = addrtype
    if ac.source_type.is_some() { rec[67] = (rec[67] & 0x0F) | 0x00; } // adsb_icao = 0

    // Validity bits
    let mut v73: u8 = 0;
    if ac.flight.is_some() { v73 |= 8; }
    if ac.alt_baro.is_some() { v73 |= 16; }
    if ac.alt_geom.is_some() { v73 |= 32; }
    if ac.lat.is_some() { v73 |= 64; }
    if ac.gs.is_some() { v73 |= 128; }
    rec[73] = v73;

    let mut v74: u8 = 0;
    if ac.track.is_some() { v74 |= 8; }
    rec[74] = v74;

    let mut v75: u8 = 0;
    if ac.baro_rate.is_some() { v75 |= 1; }
    rec[75] = v75;

    let mut v76: u8 = 0;
    if ac.squawk.is_some() { v76 |= 4; }
    rec[76] = v76;

    if let Some(ref cs) = ac.flight {
        for (i, b) in cs.as_bytes().iter().take(8).enumerate() { rec[78 + i] = *b; }
    }

    rec[104] = 1; // receiverCount

    if let Some(rssi) = ac.rssi {
        rec[105] = ((rssi + 50.0) * (255.0 / 50.0)).clamp(0.0, 255.0) as u8;
    }

    if ac.lat.is_some() { write_i32(&mut rec, 108, seen_10); }

    rec
}

fn make_header(now_ms: u64, ac_with_pos: u32, total_msgs: u32, south: i16, west: i16, north: i16, east: i16) -> [u8; 112] {
    let mut hdr = [0u8; 112];
    write_u32(&mut hdr, 0, now_ms as u32);
    write_u32(&mut hdr, 4, (now_ms >> 32) as u32);
    write_u32(&mut hdr, 8, STRIDE);
    write_u32(&mut hdr, 12, ac_with_pos);
    write_u32(&mut hdr, 16, 314159);
    write_i16(&mut hdr, 20, south);
    write_i16(&mut hdr, 22, west);
    write_i16(&mut hdr, 24, north);
    write_i16(&mut hdr, 26, east);
    write_u32(&mut hdr, 28, total_msgs);
    write_u32(&mut hdr, 40, VERSION);
    hdr
}

/// Build the full aircraft.binCraft response
pub fn build(store: &Arc<Store>) -> Vec<u8> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let now_s = now_ms as f64 / 1000.0;
    let ac_with_pos = store.map.iter().filter(|e| e.value().lat.is_some()).count() as u32;
    let total_msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed) as u32;

    let mut buf = Vec::with_capacity(STRIDE as usize + store.map.len() * STRIDE as usize);
    buf.extend_from_slice(&make_header(now_ms, ac_with_pos, total_msgs, -90, -180, 90, 180));

    for entry in store.map.iter() {
        buf.extend_from_slice(&encode_aircraft(*entry.key(), entry.value(), now_s));
    }
    buf
}

/// Build binCraft filtered by bounding box
pub fn build_filtered(store: &Arc<Store>, south: f64, north: f64, west: f64, east: f64) -> Vec<u8> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let now_s = now_ms as f64 / 1000.0;
    let ac_with_pos = store.map.iter().filter(|e| e.value().lat.is_some()).count() as u32;
    let total_msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed) as u32;

    let mut buf = Vec::with_capacity(STRIDE as usize * 4096);
    buf.extend_from_slice(&make_header(
        now_ms, ac_with_pos, total_msgs,
        south.max(-90.0) as i16, west.max(-180.0) as i16,
        north.min(90.0) as i16, east.min(180.0) as i16,
    ));

    for entry in store.map.iter() {
        let ac = entry.value();
        if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
            if lat >= south && lat <= north && lon_in_box(lon, west, east) {
                buf.extend_from_slice(&encode_aircraft(*entry.key(), ac, now_s));
            }
        }
    }
    buf
}

fn lon_in_box(lon: f64, west: f64, east: f64) -> bool {
    if west <= east { lon >= west && lon <= east }
    else { lon >= west || lon <= east }
}

fn write_u32(buf: &mut [u8], off: usize, val: u32) { buf[off..off+4].copy_from_slice(&val.to_le_bytes()); }
fn write_i32(buf: &mut [u8], off: usize, val: i32) { buf[off..off+4].copy_from_slice(&val.to_le_bytes()); }
fn write_u16(buf: &mut [u8], off: usize, val: u16) { buf[off..off+2].copy_from_slice(&val.to_le_bytes()); }
fn write_i16(buf: &mut [u8], off: usize, val: i16) { buf[off..off+2].copy_from_slice(&val.to_le_bytes()); }
