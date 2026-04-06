/// Compact binary format — variable-length, only populated fields
/// + WebSocket push with delta encoding
///
/// Wire format per aircraft:
///   u24  icao
///   u8   field_mask_0  (8 fields per byte)
///   u8   field_mask_1
///   u8   field_mask_2
///   u8   field_mask_3
///   [fields...]  only present if bit set
///
/// Field order (bit index):
///   0: lat (i32, ×1e6)       1: lon (i32, ×1e6)
///   2: alt_baro (i16, /25)   3: alt_geom (i16, /25)
///   4: gs (u16, ×10)         5: track (u16, ×10)
///   6: baro_rate (i16, /8)   7: geom_rate (i16, /8)
///   8: squawk (u16)          9: callsign (8 bytes)
///  10: category (u8)        11: ias (u16)
///  12: tas (u16)            13: mach (u16, ×1000)
///  14: mag_heading (u16,×10) 15: true_heading (u16,×10)
///  16: roll (i16, ×100)     17: track_rate (i16, ×100)
///  18: nav_alt_mcp (u16,/4) 19: nav_alt_fms (u16,/4)
///  20: nav_qnh (i16, ×10)   21: nav_heading (u16, ×10)
///  22: emergency (u8)       23: nic (u8)
///  24: nac_p (u8)           25: nac_v (u8)
///  26: sil (u8)             27: gva (u8)
///  28: sda (u8)             29: version (u8)
///  30: rssi (u8, encoded)   31: addr_type (u8)

use crate::aircraft::{Aircraft, Store};
use std::sync::Arc;

pub fn build(store: &Arc<Store>) -> Vec<u8> {
    build_inner(store, None)
}

pub fn build_filtered(store: &Arc<Store>, s: f64, n: f64, w: f64, e: f64) -> Vec<u8> {
    build_inner(store, Some((s, n, w, e)))
}

fn build_inner(store: &Arc<Store>, bbox: Option<(f64, f64, f64, f64)>) -> Vec<u8> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let now_s = now_ms as f64 / 1000.0;

    // Header: 8 bytes timestamp + 4 bytes count (filled after)
    let mut buf = Vec::with_capacity(128 * 1024);
    buf.extend_from_slice(&now_ms.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes()); // placeholder for count

    let mut count: u32 = 0;
    for entry in store.map.iter() {
        let ac = entry.value();
        if let Some((s, n, w, e)) = bbox {
            match (ac.lat, ac.lon) {
                (Some(lat), Some(lon)) if lat >= s && lat <= n && crate::bincraft::lon_in_box(lon, w, e) => {}
                _ => continue,
            }
        }
        encode_compact(*entry.key(), ac, now_s, &mut buf);
        count += 1;
    }

    // Write count
    buf[8..12].copy_from_slice(&count.to_le_bytes());
    buf
}

fn encode_compact(icao: u32, ac: &Aircraft, now_s: f64, buf: &mut Vec<u8>) {
    // ICAO (3 bytes)
    buf.push((icao >> 16) as u8);
    buf.push((icao >> 8) as u8);
    buf.push(icao as u8);

    // Build field masks
    let mut m = [0u8; 4];
    if ac.lat.is_some() { m[0] |= 1; }
    if ac.lon.is_some() { m[0] |= 2; }
    if ac.alt_baro.is_some() { m[0] |= 4; }
    if ac.alt_geom.is_some() { m[0] |= 8; }
    if ac.gs.is_some() { m[0] |= 16; }
    if ac.track.is_some() { m[0] |= 32; }
    if ac.baro_rate.is_some() { m[0] |= 64; }
    if ac.geom_rate.is_some() { m[0] |= 128; }
    if ac.squawk.is_some() { m[1] |= 1; }
    if ac.flight.is_some() { m[1] |= 2; }
    if ac.category.is_some() { m[1] |= 4; }
    if ac.ias.is_some() { m[1] |= 8; }
    if ac.tas.is_some() { m[1] |= 16; }
    if ac.mach.is_some() { m[1] |= 32; }
    if ac.mag_heading.is_some() { m[1] |= 64; }
    if ac.true_heading.is_some() { m[1] |= 128; }
    if ac.roll.is_some() { m[2] |= 1; }
    if ac.track_rate.is_some() { m[2] |= 2; }
    if ac.nav_altitude_mcp.is_some() { m[2] |= 4; }
    if ac.nav_altitude_fms.is_some() { m[2] |= 8; }
    if ac.nav_qnh.is_some() { m[2] |= 16; }
    if ac.nav_heading.is_some() { m[2] |= 32; }
    if ac.emergency.is_some() { m[2] |= 64; }
    if ac.nic.is_some() { m[2] |= 128; }
    if ac.nac_p.is_some() { m[3] |= 1; }
    if ac.nac_v.is_some() { m[3] |= 2; }
    if ac.sil.is_some() { m[3] |= 4; }
    if ac.gva.is_some() { m[3] |= 8; }
    if ac.sda.is_some() { m[3] |= 16; }
    if ac.adsb_version.is_some() { m[3] |= 32; }
    m[3] |= 64; // rssi always present
    m[3] |= 128; // seen always present

    buf.extend_from_slice(&m);

    // Fields — only if bit set
    if let Some(v) = ac.lat { buf.extend_from_slice(&((v * 1e6) as i32).to_le_bytes()); }
    if let Some(v) = ac.lon { buf.extend_from_slice(&((v * 1e6) as i32).to_le_bytes()); }
    if let Some(v) = ac.alt_baro { buf.extend_from_slice(&((v as f64 / 25.0) as i16).to_le_bytes()); }
    if let Some(v) = ac.alt_geom { buf.extend_from_slice(&((v as f64 / 25.0) as i16).to_le_bytes()); }
    if let Some(v) = ac.gs { buf.extend_from_slice(&((v * 10.0) as u16).to_le_bytes()); }
    if let Some(v) = ac.track { buf.extend_from_slice(&((v * 10.0) as u16).to_le_bytes()); }
    if let Some(v) = ac.baro_rate { buf.extend_from_slice(&((v as f64 / 8.0) as i16).to_le_bytes()); }
    if let Some(v) = ac.geom_rate { buf.extend_from_slice(&((v as f64 / 8.0) as i16).to_le_bytes()); }
    if let Some(ref v) = ac.squawk { buf.extend_from_slice(&u16::from_str_radix(v, 16).unwrap_or(0).to_le_bytes()); }
    if let Some(ref v) = ac.flight {
        let b = v.as_bytes();
        for i in 0..8 { buf.push(if i < b.len() { b[i] } else { 0 }); }
    }
    if let Some(ref v) = ac.category { buf.push(u8::from_str_radix(v, 16).unwrap_or(0)); }
    if let Some(v) = ac.ias { buf.extend_from_slice(&v.to_le_bytes()); }
    if let Some(v) = ac.tas { buf.extend_from_slice(&v.to_le_bytes()); }
    if let Some(v) = ac.mach { buf.extend_from_slice(&((v * 1000.0) as u16).to_le_bytes()); }
    if let Some(v) = ac.mag_heading { buf.extend_from_slice(&((v * 10.0) as u16).to_le_bytes()); }
    if let Some(v) = ac.true_heading { buf.extend_from_slice(&((v * 10.0) as u16).to_le_bytes()); }
    if let Some(v) = ac.roll { buf.extend_from_slice(&((v * 100.0) as i16).to_le_bytes()); }
    if let Some(v) = ac.track_rate { buf.extend_from_slice(&((v * 100.0) as i16).to_le_bytes()); }
    if let Some(v) = ac.nav_altitude_mcp { buf.extend_from_slice(&((v / 4) as u16).to_le_bytes()); }
    if let Some(v) = ac.nav_altitude_fms { buf.extend_from_slice(&((v / 4) as u16).to_le_bytes()); }
    if let Some(v) = ac.nav_qnh { buf.extend_from_slice(&((v * 10.0) as i16).to_le_bytes()); }
    if let Some(v) = ac.nav_heading { buf.extend_from_slice(&((v * 10.0) as u16).to_le_bytes()); }
    if let Some(v) = ac.emergency { buf.push(v); }
    if let Some(v) = ac.nic { buf.push(v); }
    if let Some(v) = ac.nac_p { buf.push(v); }
    if let Some(v) = ac.nac_v { buf.push(v); }
    if let Some(v) = ac.sil { buf.push(v); }
    if let Some(v) = ac.gva { buf.push(v); }
    if let Some(v) = ac.sda { buf.push(v); }
    if let Some(v) = ac.adsb_version { buf.push(v); }
    // rssi
    buf.push(ac.rssi.map(|r| ((r + 50.0) * (255.0 / 50.0)).clamp(0.0, 255.0) as u8).unwrap_or(0));
    // seen (u16, tenths of seconds)
    buf.extend_from_slice(&(((now_s - ac.last_update) * 10.0) as u16).to_le_bytes());
}
