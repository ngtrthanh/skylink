/// GeoJSON FeatureCollection output — ready for MapLibre GL JS source.setData()

use crate::aircraft::Store;
use std::sync::Arc;

pub fn build(store: &Arc<Store>) -> Vec<u8> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    let mut buf = Vec::with_capacity(512 * 1024);
    buf.extend_from_slice(b"{\"type\":\"FeatureCollection\",\"features\":[");

    let mut first = true;
    for entry in store.map.iter() {
        let ac = entry.value();
        let (lat, lon) = match (ac.lat, ac.lon) {
            (Some(la), Some(lo)) if ac.last_pos_update > 0.0 && (now - ac.last_pos_update) < 60.0 => (la, lo),
            _ => continue,
        };

        if !first { buf.push(b','); }
        first = false;

        // Geometry
        buf.extend_from_slice(b"{\"type\":\"Feature\",\"geometry\":{\"type\":\"Point\",\"coordinates\":[");
        write_f6(&mut buf, lon); buf.push(b','); write_f6(&mut buf, lat);
        buf.extend_from_slice(b"]},\"properties\":{");

        // Properties — only non-null fields
        write_str(&mut buf, "hex", &ac.hex);
        if let Some(ref v) = ac.flight { buf.push(b','); write_str(&mut buf, "flight", v); }
        if let Some(v) = ac.alt_baro { buf.push(b','); write_kv_int(&mut buf, "alt_baro", v); }
        if let Some(v) = ac.alt_geom { buf.push(b','); write_kv_int(&mut buf, "alt_geom", v); }
        if let Some(v) = ac.gs { buf.push(b','); write_kv_f1(&mut buf, "gs", v); }
        if let Some(v) = ac.track { buf.push(b','); write_kv_f1(&mut buf, "track", v); }
        if let Some(v) = ac.baro_rate { buf.push(b','); write_kv_int(&mut buf, "baro_rate", v); }
        if let Some(ref v) = ac.squawk { buf.push(b','); write_str(&mut buf, "squawk", v); }
        if let Some(ref v) = ac.category { buf.push(b','); write_str(&mut buf, "category", v); }
        if let Some(v) = ac.mag_heading { buf.push(b','); write_kv_f1(&mut buf, "mag_heading", v); }
        if let Some(v) = ac.ias { buf.push(b','); write_kv_int(&mut buf, "ias", v as i32); }
        if let Some(v) = ac.mach { buf.push(b','); write_kv_f3(&mut buf, "mach", v); }
        if let Some(ref v) = ac.source_type { buf.push(b','); write_str(&mut buf, "type", v); }
        // seen
        buf.push(b','); write_kv_f1(&mut buf, "seen", now - ac.last_update);
        buf.push(b','); write_kv_f1(&mut buf, "seen_pos", now - ac.last_pos_update);
        buf.push(b','); write_kv_int(&mut buf, "messages", ac.messages as i32);

        buf.extend_from_slice(b"}}");
    }

    buf.extend_from_slice(b"]}");
    buf
}

pub fn build_filtered(store: &Arc<Store>, south: f64, north: f64, west: f64, east: f64) -> Vec<u8> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    let mut buf = Vec::with_capacity(256 * 1024);
    buf.extend_from_slice(b"{\"type\":\"FeatureCollection\",\"features\":[");

    let mut first = true;
    for entry in store.map.iter() {
        let ac = entry.value();
        let (lat, lon) = match (ac.lat, ac.lon) {
            (Some(la), Some(lo)) if ac.last_pos_update > 0.0 && (now - ac.last_pos_update) < 60.0
                && la >= south && la <= north && crate::bincraft::lon_in_box(lo, west, east) => (la, lo),
            _ => continue,
        };

        if !first { buf.push(b','); }
        first = false;

        buf.extend_from_slice(b"{\"type\":\"Feature\",\"geometry\":{\"type\":\"Point\",\"coordinates\":[");
        write_f6(&mut buf, lon); buf.push(b','); write_f6(&mut buf, lat);
        buf.extend_from_slice(b"]},\"properties\":{");
        write_str(&mut buf, "hex", &ac.hex);
        if let Some(ref v) = ac.flight { buf.push(b','); write_str(&mut buf, "flight", v); }
        if let Some(v) = ac.alt_baro { buf.push(b','); write_kv_int(&mut buf, "alt_baro", v); }
        if let Some(v) = ac.gs { buf.push(b','); write_kv_f1(&mut buf, "gs", v); }
        if let Some(v) = ac.track { buf.push(b','); write_kv_f1(&mut buf, "track", v); }
        if let Some(ref v) = ac.squawk { buf.push(b','); write_str(&mut buf, "squawk", v); }
        if let Some(ref v) = ac.category { buf.push(b','); write_str(&mut buf, "category", v); }
        buf.push(b','); write_kv_f1(&mut buf, "seen", now - ac.last_update);
        buf.extend_from_slice(b"}}");
    }

    buf.extend_from_slice(b"]}");
    buf
}

// Fast writers
fn write_f6(buf: &mut Vec<u8>, v: f64) { buf.extend_from_slice(format!("{:.6}", v).as_bytes()); }
fn write_kv_f1(buf: &mut Vec<u8>, k: &str, v: f64) {
    buf.push(b'"'); buf.extend_from_slice(k.as_bytes()); buf.extend_from_slice(b"\":");
    buf.extend_from_slice(format!("{:.1}", v).as_bytes());
}
fn write_kv_f3(buf: &mut Vec<u8>, k: &str, v: f64) {
    buf.push(b'"'); buf.extend_from_slice(k.as_bytes()); buf.extend_from_slice(b"\":");
    buf.extend_from_slice(format!("{:.3}", v).as_bytes());
}
fn write_kv_int(buf: &mut Vec<u8>, k: &str, v: i32) {
    buf.push(b'"'); buf.extend_from_slice(k.as_bytes()); buf.extend_from_slice(b"\":");
    buf.extend_from_slice(v.to_string().as_bytes());
}
fn write_str(buf: &mut Vec<u8>, k: &str, v: &str) {
    buf.push(b'"'); buf.extend_from_slice(k.as_bytes()); buf.extend_from_slice(b"\":\"");
    buf.extend_from_slice(v.as_bytes()); buf.push(b'"');
}
