/// binCraft binary format — wire-compatible with readsb
/// Struct: 112 bytes per aircraft, little-endian, packed

use crate::aircraft::{Aircraft, Store};
use std::sync::Arc;

const STRIDE: u32 = 112;
const VERSION: u32 = 20250403;

pub fn encode_aircraft_pub(icao: u32, ac: &Aircraft, now_s: f64) -> [u8; 112] {
    encode_aircraft(icao, ac, now_s)
}

fn encode_aircraft(icao: u32, ac: &Aircraft, now_s: f64) -> [u8; 112] {
    let mut rec = [0u8; 112];
    let seen_10 = ((now_s - ac.last_update) * 10.0) as i32;

    write_u32(&mut rec, 0, icao);
    write_i32(&mut rec, 4, seen_10);

    if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
        if ac.last_pos_update > 0.0 && (now_s - ac.last_pos_update) < 60.0 {
            write_i32(&mut rec, 8, (lon * 1e6) as i32);
            write_i32(&mut rec, 12, (lat * 1e6) as i32);
        }
    }

    if let Some(vr) = ac.baro_rate { write_i16(&mut rec, 16, (vr as f64 / 8.0) as i16); }
    if let Some(vr) = ac.geom_rate { write_i16(&mut rec, 18, (vr as f64 / 8.0) as i16); }
    if let Some(alt) = ac.alt_baro { write_i16(&mut rec, 20, (alt as f64 / 25.0) as i16); }
    if let Some(alt) = ac.alt_geom { write_i16(&mut rec, 22, (alt as f64 / 25.0) as i16); }
    if let Some(v) = ac.nav_altitude_mcp { write_u16(&mut rec, 24, (v / 4) as u16); }
    if let Some(v) = ac.nav_altitude_fms { write_u16(&mut rec, 26, (v / 4) as u16); }
    if let Some(v) = ac.nav_qnh { write_i16(&mut rec, 28, (v * 10.0) as i16); }
    if let Some(v) = ac.nav_heading { write_i16(&mut rec, 30, (v * 90.0) as i16); }

    if let Some(ref sq) = ac.squawk {
        if let Ok(v) = u16::from_str_radix(sq, 16) { write_u16(&mut rec, 32, v); }
    }

    if let Some(gs) = ac.gs { write_i16(&mut rec, 34, (gs * 10.0) as i16); }
    if let Some(v) = ac.mach { write_i16(&mut rec, 36, (v * 1000.0) as i16); }
    if let Some(v) = ac.roll { write_i16(&mut rec, 38, (v * 100.0) as i16); }
    if let Some(trk) = ac.track { write_i16(&mut rec, 40, (trk * 90.0) as i16); }
    if let Some(v) = ac.track_rate { write_i16(&mut rec, 42, (v * 100.0) as i16); }
    if let Some(v) = ac.mag_heading { write_i16(&mut rec, 44, (v * 90.0) as i16); }
    if let Some(v) = ac.true_heading { write_i16(&mut rec, 46, (v * 90.0) as i16); }
    if let Some(v) = ac.tas { write_u16(&mut rec, 56, v); }
    if let Some(v) = ac.ias { write_u16(&mut rec, 58, v); }

    write_u16(&mut rec, 62, ac.messages.min(65535) as u16);

    if let Some(ref cat) = ac.category {
        if let Ok(v) = u8::from_str_radix(cat, 16) { rec[64] = v; }
    }
    if let Some(v) = ac.nic { rec[65] = v; }
    if let Some(v) = ac.emergency { rec[67] = (rec[67] & 0xF0) | (v & 0x0F); }
    rec[67] = (rec[67] & 0x0F) | ((ac.addr_type & 0x0F) << 4);
    if let Some(v) = ac.adsb_version { rec[69] = (rec[69] & 0x0F) | ((v & 0x0F) << 4); }
    if let Some(v) = ac.nac_p { rec[71] = (rec[71] & 0xF0) | (v & 0x0F); }
    if let Some(v) = ac.nac_v { rec[71] = (rec[71] & 0x0F) | ((v & 0x0F) << 4); }
    if let Some(v) = ac.sil { rec[72] = (rec[72] & 0xFC) | (v & 0x03); }
    if let Some(v) = ac.gva { rec[72] = (rec[72] & 0xF3) | ((v & 0x03) << 2); }
    if let Some(v) = ac.sda { rec[72] = (rec[72] & 0xCF) | ((v & 0x03) << 4); }

    // Validity bits
    let mut v73: u8 = 0;
    if let Some(v) = ac.nic_baro { v73 |= v & 1; }
    let pos_fresh = ac.lat.is_some() && ac.last_pos_update > 0.0 && (now_s - ac.last_pos_update) < 60.0;
    if ac.flight.is_some() { v73 |= 8; }
    if ac.alt_baro.is_some() { v73 |= 16; }
    if ac.alt_geom.is_some() { v73 |= 32; }
    if pos_fresh { v73 |= 64; }
    if ac.gs.is_some() { v73 |= 128; }
    rec[73] = v73;

    let mut v74: u8 = 0;
    if ac.ias.is_some() { v74 |= 1; }
    if ac.tas.is_some() { v74 |= 2; }
    if ac.mach.is_some() { v74 |= 4; }
    if ac.track.is_some() { v74 |= 8; }
    if ac.track_rate.is_some() { v74 |= 16; }
    if ac.roll.is_some() { v74 |= 32; }
    if ac.mag_heading.is_some() { v74 |= 64; }
    if ac.true_heading.is_some() { v74 |= 128; }
    rec[74] = v74;

    let mut v75: u8 = 0;
    if ac.baro_rate.is_some() { v75 |= 1; }
    if ac.geom_rate.is_some() { v75 |= 2; }
    if ac.nac_p.is_some() { v75 |= 32; }
    if ac.nac_v.is_some() { v75 |= 64; }
    if ac.sil.is_some() { v75 |= 128; }
    rec[75] = v75;

    let mut v76: u8 = 0;
    if ac.gva.is_some() { v76 |= 1; }
    if ac.sda.is_some() { v76 |= 2; }
    if ac.squawk.is_some() { v76 |= 4; }
    if ac.emergency.is_some() { v76 |= 8; }
    if ac.nav_qnh.is_some() { v76 |= 32; }
    if ac.nav_altitude_mcp.is_some() { v76 |= 64; }
    if ac.nav_altitude_fms.is_some() { v76 |= 128; }
    rec[76] = v76;

    let mut v77: u8 = 0;
    if ac.nav_heading.is_some() { v77 |= 2; }
    if ac.nav_modes.is_some() { v77 |= 4; }
    rec[77] = v77;

    if let Some(ref cs) = ac.flight {
        for (i, b) in cs.as_bytes().iter().take(8).enumerate() { rec[78 + i] = *b; }
    }

    // type designator (bytes 88-91) and registration (bytes 92-103)
    if let Some(ref t) = ac.t {
        for (i, b) in t.as_bytes().iter().take(4).enumerate() { rec[88 + i] = *b; }
    }
    if let Some(ref r) = ac.r {
        for (i, b) in r.as_bytes().iter().take(12).enumerate() { rec[92 + i] = *b; }
    }

    rec[104] = 1; // receiverCount

    if let Some(rssi) = ac.rssi {
        rec[105] = ((rssi + 50.0) * (255.0 / 50.0)).clamp(0.0, 255.0) as u8;
    }

    if pos_fresh { write_i32(&mut rec, 108, ((now_s - ac.last_pos_update) * 10.0) as i32); }

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

pub fn lon_in_box(lon: f64, west: f64, east: f64) -> bool {
    if west <= east { lon >= west && lon <= east }
    else { lon >= west || lon <= east }
}

fn write_u32(buf: &mut [u8], off: usize, val: u32) { buf[off..off+4].copy_from_slice(&val.to_le_bytes()); }
fn write_i32(buf: &mut [u8], off: usize, val: i32) { buf[off..off+4].copy_from_slice(&val.to_le_bytes()); }
fn write_u16(buf: &mut [u8], off: usize, val: u16) { buf[off..off+2].copy_from_slice(&val.to_le_bytes()); }
fn write_i16(buf: &mut [u8], off: usize, val: i16) { buf[off..off+2].copy_from_slice(&val.to_le_bytes()); }
