/// binVessel — compact binary vessel format (64-byte stride)
/// Same philosophy as binCraft: pre-encode once, zero-copy bbox filter
///
/// Header (64 bytes): timestamp, vessel count, bbox, version
/// Per vessel (64 bytes):
///   0:4   mmsi (u32)
///   4:8   lat (i32 × 1e6)
///   8:12  lon (i32 × 1e6)
///  12:14  sog (u16 × 10)
///  14:16  cog (u16 × 10)
///  16:18  heading (u16)
///  18     status (u8)
///  19     shiptype (u8)
///  20     shipclass (u8)
///  21     validity bits
///  22:24  turn (i16)
///  24:28  imo (u32)
///  28:48  shipname (20 bytes)
///  48:55  callsign (7 bytes)
///  55     country_mid (u8)
///  56:58  to_bow (u16)
///  58:60  to_stern (u16)
///  60     draught (u8 × 10)
///  61     altitude_low (u8, for SAR)
///  62:64  last_signal (u16, seconds ago × 10)

use crate::ais::vessel::{Vessel, VesselStore};
use std::sync::Arc;

pub const STRIDE: usize = 64;
const VERSION: u32 = 20260407;

fn write_u32(b: &mut [u8], o: usize, v: u32) { b[o..o+4].copy_from_slice(&v.to_le_bytes()); }
fn write_i32(b: &mut [u8], o: usize, v: i32) { b[o..o+4].copy_from_slice(&v.to_le_bytes()); }
fn write_u16(b: &mut [u8], o: usize, v: u16) { b[o..o+2].copy_from_slice(&v.to_le_bytes()); }
fn write_i16(b: &mut [u8], o: usize, v: i16) { b[o..o+2].copy_from_slice(&v.to_le_bytes()); }

fn encode_vessel(v: &Vessel, now: f64) -> [u8; STRIDE] {
    let mut r = [0u8; STRIDE];
    write_u32(&mut r, 0, v.mmsi);

    let has_pos = v.lat.is_some() && v.lon.is_some();
    if has_pos {
        write_i32(&mut r, 4, (v.lat.unwrap() as f64 * 1e6) as i32);
        write_i32(&mut r, 8, (v.lon.unwrap() as f64 * 1e6) as i32);
    }

    let sog = v.speed.map(|s| (s * 10.0) as u16).unwrap_or(0xFFFF);
    let cog = v.cog.map(|c| (c * 10.0) as u16).unwrap_or(0xFFFF);
    let hdg = v.heading.unwrap_or(0xFFFF);
    write_u16(&mut r, 12, sog);
    write_u16(&mut r, 14, cog);
    write_u16(&mut r, 16, hdg);
    r[18] = v.status.unwrap_or(15);
    r[19] = v.shiptype;
    r[20] = v.shipclass;

    // Validity bits
    let mut vb: u8 = 0;
    if has_pos { vb |= 1; }
    if v.speed.is_some() { vb |= 2; }
    if v.cog.is_some() { vb |= 4; }
    if v.heading.is_some() { vb |= 8; }
    if v.status.is_some() { vb |= 16; }
    if !v.shipname.is_empty() { vb |= 32; }
    if v.imo.is_some() { vb |= 64; }
    if !v.callsign.is_empty() { vb |= 128; }
    r[21] = vb;

    write_i16(&mut r, 22, v.turn.unwrap_or(0));
    write_u32(&mut r, 24, v.imo.unwrap_or(0));

    // shipname (20 bytes)
    for (i, b) in v.shipname.as_bytes().iter().take(20).enumerate() { r[28 + i] = *b; }
    // callsign (7 bytes)
    for (i, b) in v.callsign.as_bytes().iter().take(7).enumerate() { r[48 + i] = *b; }

    // country MID
    let mid = (v.mmsi / 1000000) % 1000;
    r[55] = (mid / 10) as u8;

    write_u16(&mut r, 56, v.to_bow.unwrap_or(0));
    write_u16(&mut r, 58, v.to_stern.unwrap_or(0));
    r[60] = v.draught.map(|d| (d * 10.0).min(255.0) as u8).unwrap_or(0);
    r[61] = v.altitude.map(|a| a.min(255) as u8).unwrap_or(0);

    let age = ((now - v.last_signal) * 10.0).min(65535.0) as u16;
    write_u16(&mut r, 62, age);

    r
}

fn make_header(now_ms: u64, count: u32, south: i16, west: i16, north: i16, east: i16) -> [u8; STRIDE] {
    let mut h = [0u8; STRIDE];
    write_u32(&mut h, 0, (now_ms & 0xFFFFFFFF) as u32);
    write_u32(&mut h, 4, (now_ms >> 32) as u32);
    write_u32(&mut h, 8, STRIDE as u32);
    write_u32(&mut h, 12, count);
    write_u32(&mut h, 16, VERSION);
    write_i16(&mut h, 20, south);
    write_i16(&mut h, 22, west);
    write_i16(&mut h, 24, north);
    write_i16(&mut h, 26, east);
    h
}

/// Build full binVessel buffer
pub fn build(store: &Arc<VesselStore>) -> Vec<u8> {
    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let now_s = now_ms as f64 / 1000.0;
    let count = store.map.iter().filter(|e| now_s - e.value().last_signal <= 600.0).count() as u32;

    let mut buf = Vec::with_capacity(STRIDE + store.map.len() * STRIDE);
    buf.extend_from_slice(&make_header(now_ms, count, -90, -180, 90, 180));

    for entry in store.map.iter() {
        let v = entry.value();
        if now_s - v.last_signal > 600.0 { continue; }
        buf.extend_from_slice(&encode_vessel(v, now_s));
    }
    buf
}

/// Zero-copy bbox filter from pre-built cache
pub fn build_filtered_from_cache(cache: &[u8], south: f64, north: f64, west: f64, east: f64) -> Vec<u8> {
    if cache.len() < STRIDE { return Vec::new(); }

    let s_i32 = (south * 1e6) as i32;
    let n_i32 = (north * 1e6) as i32;
    let w_i32 = (west * 1e6) as i32;
    let e_i32 = (east * 1e6) as i32;

    let mut hdr = [0u8; STRIDE];
    hdr.copy_from_slice(&cache[..STRIDE]);
    write_i16(&mut hdr, 20, south.max(-90.0) as i16);
    write_i16(&mut hdr, 22, west.max(-180.0) as i16);
    write_i16(&mut hdr, 24, north.min(90.0) as i16);
    write_i16(&mut hdr, 26, east.min(180.0) as i16);

    let mut buf = Vec::with_capacity(cache.len());
    buf.extend_from_slice(&hdr);

    let mut off = STRIDE;
    while off + STRIDE <= cache.len() {
        let rec = &cache[off..off + STRIDE];
        let vb = rec[21];
        if vb & 1 != 0 { // has position
            let lat = i32::from_le_bytes([rec[4], rec[5], rec[6], rec[7]]);
            let lon = i32::from_le_bytes([rec[8], rec[9], rec[10], rec[11]]);
            if lat >= s_i32 && lat <= n_i32 {
                let lon_ok = if w_i32 <= e_i32 { lon >= w_i32 && lon <= e_i32 } else { lon >= w_i32 || lon <= e_i32 };
                if lon_ok { buf.extend_from_slice(rec); }
            }
        }
        off += STRIDE;
    }
    // Update count in header
    let count = ((buf.len() - STRIDE) / STRIDE) as u32;
    write_u32(&mut buf[..STRIDE], 12, count);
    buf
}
