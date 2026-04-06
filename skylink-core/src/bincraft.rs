/// binCraft binary format — wire-compatible with readsb
/// Struct: 112 bytes per aircraft, little-endian, packed

use crate::aircraft::Store;
use std::sync::Arc;

const STRIDE: u32 = 112;
const VERSION: u32 = 20250403;

/// Build the full aircraft.binCraft response
pub fn build(store: &Arc<Store>) -> Vec<u8> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;

    let ac_count = store.map.len();
    let ac_with_pos = store.map.iter().filter(|e| e.value().lat.is_some()).count() as u32;
    let total_msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed) as u32;

    let mut buf = Vec::with_capacity(STRIDE as usize + ac_count * STRIDE as usize);

    // Header (112 bytes, same stride as aircraft entries)
    let mut hdr = [0u8; 112];
    // u32[0..1] = now_ms as two u32 (little-endian i64)
    write_u32(&mut hdr, 0, now_ms as u32);
    write_u32(&mut hdr, 4, (now_ms >> 32) as u32);
    // u32[2] = elementSize (stride)
    write_u32(&mut hdr, 8, STRIDE);
    // u32[3] = ac_count_with_pos
    write_u32(&mut hdr, 12, ac_with_pos);
    // u32[4] = globe_index (314159 for all-aircraft)
    write_u32(&mut hdr, 16, 314159);
    // i16[10..13] = south, west, north, east
    write_i16(&mut hdr, 20, -90);
    write_i16(&mut hdr, 22, -180);
    write_i16(&mut hdr, 24, 90);
    write_i16(&mut hdr, 26, 180);
    // u32[7] = messageCount
    write_u32(&mut hdr, 28, total_msgs);
    // s32[8] = receiver_lat, s32[9] = receiver_lon (0 = not set)
    // u32[10] = binCraftVersion
    write_u32(&mut hdr, 40, VERSION);
    // u32[11] = messageRate (×10)
    write_u32(&mut hdr, 44, 0);
    // u32[12] = flags
    write_u32(&mut hdr, 48, 0);

    buf.extend_from_slice(&hdr);

    // Aircraft entries
    let now_s = now_ms as f64 / 1000.0;
    for entry in store.map.iter() {
        let ac = entry.value();
        let mut rec = [0u8; 112];

        // s32[0] = hex (24-bit ICAO)
        let icao = *entry.key();
        write_u32(&mut rec, 0, icao);

        // s32[1] = seen (in tenths of seconds)
        let seen_10 = ((now_s - ac.last_update) * 10.0) as i32;
        write_i32(&mut rec, 4, seen_10);

        // s32[2] = lon * 1e6, s32[3] = lat * 1e6
        if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
            write_i32(&mut rec, 8, (lon * 1e6) as i32);
            write_i32(&mut rec, 12, (lat * 1e6) as i32);
        }

        // s16[8] = baro_rate / 8
        if let Some(vr) = ac.baro_rate { write_i16(&mut rec, 16, (vr as f64 / 8.0) as i16); }
        // s16[9] = geom_rate / 8 (not tracked yet)

        // s16[10] = baro_alt / 25
        if let Some(alt) = ac.alt_baro { write_i16(&mut rec, 20, (alt as f64 / 25.0) as i16); }
        // s16[11] = geom_alt / 25
        if let Some(alt) = ac.alt_geom { write_i16(&mut rec, 22, (alt as f64 / 25.0) as i16); }

        // u16[16] = squawk (as u16, BCD)
        if let Some(ref sq) = ac.squawk {
            if let Ok(v) = u16::from_str_radix(sq, 16) {
                write_u16(&mut rec, 32, v);
            }
        }

        // s16[17] = gs * 10
        if let Some(gs) = ac.gs { write_i16(&mut rec, 34, (gs * 10.0) as i16); }

        // s16[20] = track * 90
        if let Some(trk) = ac.track { write_i16(&mut rec, 40, (trk * 90.0) as i16); }

        // u16[31] = messages
        write_u16(&mut rec, 62, ac.messages.min(65535) as u16);

        // u8[64] = category
        if let Some(ref cat) = ac.category {
            if let Ok(v) = u8::from_str_radix(cat, 16) {
                rec[64] = v;
            }
        }

        // u8[73] validity bits
        let mut v73: u8 = 0;
        if ac.flight.is_some() { v73 |= 8; }
        if ac.alt_baro.is_some() { v73 |= 16; }
        if ac.alt_geom.is_some() { v73 |= 32; }
        if ac.lat.is_some() { v73 |= 64; }
        if ac.gs.is_some() { v73 |= 128; }
        rec[73] = v73;

        // u8[74] validity bits
        let mut v74: u8 = 0;
        if ac.track.is_some() { v74 |= 8; }
        if ac.baro_rate.is_some() { v74 |= 1; } // wait, baro_rate is u8[75] bit 0
        rec[74] = v74;

        // u8[75] validity bits
        let mut v75: u8 = 0;
        if ac.baro_rate.is_some() { v75 |= 1; }
        rec[75] = v75;

        // u8[76] validity bits
        let mut v76: u8 = 0;
        if ac.squawk.is_some() { v76 |= 4; }
        rec[76] = v76;

        // callsign at bytes 78-85
        if let Some(ref cs) = ac.flight {
            let cs_bytes = cs.as_bytes();
            for i in 0..cs_bytes.len().min(8) {
                rec[78 + i] = cs_bytes[i];
            }
        }

        // u8[104] = receiverCount
        rec[104] = 1;

        // u8[105] = signal
        if let Some(rssi) = ac.rssi {
            // reverse: rssi = (u8 * 50/255) - 50 → u8 = (rssi + 50) * 255/50
            let sig = ((rssi + 50.0) * (255.0 / 50.0)).clamp(0.0, 255.0) as u8;
            rec[105] = sig;
        }

        // s32[27] = seen_pos (in tenths of seconds)
        if ac.lat.is_some() {
            write_i32(&mut rec, 108, seen_10);
        }

        buf.extend_from_slice(&rec);
    }

    buf
}

fn write_u32(buf: &mut [u8], off: usize, val: u32) {
    buf[off..off+4].copy_from_slice(&val.to_le_bytes());
}
fn write_i32(buf: &mut [u8], off: usize, val: i32) {
    buf[off..off+4].copy_from_slice(&val.to_le_bytes());
}
fn write_u16(buf: &mut [u8], off: usize, val: u16) {
    buf[off..off+2].copy_from_slice(&val.to_le_bytes());
}
fn write_i16(buf: &mut [u8], off: usize, val: i16) {
    buf[off..off+2].copy_from_slice(&val.to_le_bytes());
}
