/// Aircraft state store with pre-built JSON cache

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::Serialize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tracing::info;

use crate::mode_s::Message;

fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64()
}

#[derive(Debug, Clone, Serialize)]
pub struct Aircraft {
    pub hex: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub flight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub alt_baro: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub alt_geom: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub gs: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub track: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub baro_rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub geom_rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub squawk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub lon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub seen_pos: Option<f64>,
    pub seen: f64,
    #[serde(skip_serializing_if = "Option::is_none")] pub rssi: Option<f64>,
    pub messages: u64,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")] pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub ias: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")] pub tas: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")] pub mach: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub mag_heading: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub true_heading: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub roll: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub track_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub nav_altitude_mcp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub nav_altitude_fms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub nav_qnh: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub nav_heading: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub emergency: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub nic: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub nac_p: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub nac_v: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub sil: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub sil_type: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub gva: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub sda: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub nic_baro: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub adsb_version: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")] pub nav_modes: Option<u8>,
    #[serde(skip)] pub addr_type: u8,
    /// Type designator from aircraft DB (e.g. "B738", "A320")
    #[serde(skip_serializing_if = "Option::is_none")] pub r: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub t: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub wtc: Option<String>,

    #[serde(skip)] pub cpr_even: Option<(u32, u32, f64)>,
    #[serde(skip)] pub cpr_odd: Option<(u32, u32, f64)>,
    #[serde(skip)] pub last_update: f64,
    #[serde(skip)] pub last_pos_update: f64,
    #[serde(skip)] pub trace: Vec<TracePoint>,
}

#[derive(Debug, Clone)]
pub struct TracePoint {
    pub ts: f64,
    pub lat: f64,
    pub lon: f64,
    pub alt_baro: Option<i32>,
    pub alt_geom: Option<i32>,
    pub gs: Option<f64>,
    pub track: Option<f64>,
    pub baro_rate: Option<i32>,
    pub ias: Option<u16>,
}

pub struct Store {
    pub map: DashMap<u32, Aircraft>,
    /// Aircraft database for type lookups
    pub db: crate::db::AircraftDb,
    /// Pre-built JSON response — updated every ~1s by json_builder
    pub json_cache: RwLock<bytes::Bytes>,
    /// Pre-built binCraft response — updated every ~1s
    pub bincraft_cache: RwLock<bytes::Bytes>,
    /// Pre-built zstd-compressed caches
    pub json_zstd_cache: RwLock<bytes::Bytes>,
    pub bincraft_zstd_cache: RwLock<bytes::Bytes>,
    pub pb_zstd_cache: RwLock<bytes::Bytes>,
    pub compact_cache: RwLock<bytes::Bytes>,
    pub compact_zstd_cache: RwLock<bytes::Bytes>,
    pub geojson_cache: RwLock<bytes::Bytes>,
    pub geojson_zstd_cache: RwLock<bytes::Bytes>,
    /// Pre-built protobuf response
    pub pb_cache: RwLock<bytes::Bytes>,
    pub messages_total: std::sync::atomic::AtomicU64,
    pub clients: RwLock<Vec<ClientInfo>>,
    pub receivers_cache: RwLock<bytes::Bytes>,
    pub start_time: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClientInfo {
    pub addr: String,
    pub connected_at: f64,
    pub messages: u64,
}

impl Store {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
        Self {
            map: DashMap::with_capacity(16384),
            db: crate::db::AircraftDb::load(),
            json_cache: RwLock::new(bytes::Bytes::from_static(b"{\"now\":0,\"messages\":0,\"aircraft\":[]}")),
            bincraft_cache: RwLock::new(bytes::Bytes::new()),
            json_zstd_cache: RwLock::new(bytes::Bytes::new()),
            bincraft_zstd_cache: RwLock::new(bytes::Bytes::new()),
            pb_zstd_cache: RwLock::new(bytes::Bytes::new()),
            compact_cache: RwLock::new(bytes::Bytes::new()),
            compact_zstd_cache: RwLock::new(bytes::Bytes::new()),
            geojson_cache: RwLock::new(bytes::Bytes::new()),
            geojson_zstd_cache: RwLock::new(bytes::Bytes::new()),
            pb_cache: RwLock::new(bytes::Bytes::new()),
            messages_total: std::sync::atomic::AtomicU64::new(0),
            clients: RwLock::new(Vec::new()),
            receivers_cache: RwLock::new(bytes::Bytes::from_static(b"[]")),
            start_time: now,
        }
    }

    pub fn update_from_message(&self, msg: &Message, signal: u8) {
        if msg.icao == 0 { return; }
        let t = now();
        self.messages_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let hex_str = format!("{:06x}", msg.icao);
        let db_info = self.db.aircraft.get(&hex_str);
        let (r_val, t_val, desc_val, wtc_val) = match db_info {
            Some((reg, td)) => {
                let (d, w) = self.db.get_type_info(td).unwrap_or(("", ""));
                (Some(reg.clone()), Some(td.clone()),
                 if d.is_empty() { None } else { Some(d.to_string()) },
                 if w.is_empty() { None } else { Some(w.to_string()) })
            }
            None => (None, None, None, None),
        };

        let mut entry = self.map.entry(msg.icao).or_insert_with(|| Aircraft {
            hex: hex_str,
            flight: None, alt_baro: None, alt_geom: None, gs: None, track: None,
            baro_rate: None, geom_rate: None, squawk: None, category: None,
            lat: None, lon: None, seen_pos: None, seen: 0.0, rssi: None,
            messages: 0, source_type: None,
            ias: None, tas: None, mach: None, mag_heading: None, true_heading: None,
            roll: None, track_rate: None,
            nav_altitude_mcp: None, nav_altitude_fms: None, nav_qnh: None,
            nav_heading: None, emergency: None, nav_modes: None,
            nic: None, nac_p: None, nac_v: None, sil: None, sil_type: None,
            gva: None, sda: None, nic_baro: None, adsb_version: None,
            addr_type: 0,
            r: r_val, t: t_val, desc: desc_val, wtc: wtc_val,
            cpr_even: None, cpr_odd: None, last_update: t, last_pos_update: 0.0,
            trace: Vec::new(),
        });
        let ac = entry.value_mut();

        ac.messages += 1;
        ac.last_update = t;
        ac.rssi = Some(10.0 * ((signal as f64 / 255.0).powi(2) + 1e-12).log10());

        if let Some(alt) = msg.altitude { ac.alt_baro = Some(alt); }
        if let Some(alt) = msg.alt_gnss { ac.alt_geom = Some(alt); }
        if let Some(gs) = msg.ground_speed { ac.gs = Some((gs * 10.0).round() / 10.0); }
        if let Some(trk) = msg.ground_track { ac.track = Some((trk * 10.0).round() / 10.0); }
        if let Some(vr) = msg.vert_rate { ac.baro_rate = Some(vr); }
        if let Some(vr) = msg.geom_rate { ac.geom_rate = Some(vr); }
        if let Some(sq) = msg.squawk { ac.squawk = Some(format!("{:04o}", sq)); }
        if let Some(ref cs) = msg.callsign { ac.flight = Some(cs.clone()); }
        if let Some(cat) = msg.category {
            let tc = msg.df;
            ac.category = Some(format!("{:02X}", ((0x0E - (tc / 4)) << 4) | cat));
        }
        if let Some(v) = msg.ias { ac.ias = Some(v); }
        if let Some(v) = msg.tas { ac.tas = Some(v); }
        if let Some(v) = msg.mach { ac.mach = Some(v); }
        if let Some(v) = msg.mag_heading { ac.mag_heading = Some((v * 10.0).round() / 10.0); }
        if let Some(v) = msg.true_heading { ac.true_heading = Some((v * 10.0).round() / 10.0); }
        if let Some(v) = msg.roll { ac.roll = Some((v * 10.0).round() / 10.0); }
        if let Some(v) = msg.track_rate { ac.track_rate = Some((v * 100.0).round() / 100.0); }
        if let Some(v) = msg.nav_altitude_mcp { ac.nav_altitude_mcp = Some(v); }
        if let Some(v) = msg.nav_altitude_fms { ac.nav_altitude_fms = Some(v); }
        if let Some(v) = msg.nav_qnh { ac.nav_qnh = Some((v * 10.0).round() / 10.0); }
        if let Some(v) = msg.nav_heading { ac.nav_heading = Some((v * 10.0).round() / 10.0); }
        if let Some(v) = msg.emergency { ac.emergency = Some(v); }
        if let Some(v) = msg.nic { ac.nic = Some(v); }
        if let Some(v) = msg.nac_p { ac.nac_p = Some(v); }
        if let Some(v) = msg.nac_v { ac.nac_v = Some(v); }
        if let Some(v) = msg.sil { ac.sil = Some(v); }
        if let Some(v) = msg.sil_type { ac.sil_type = Some(v); }
        if let Some(v) = msg.gva { ac.gva = Some(v); }
        if let Some(v) = msg.sda { ac.sda = Some(v); }
        if let Some(v) = msg.nic_baro { ac.nic_baro = Some(v); }
        if let Some(v) = msg.adsb_version { ac.adsb_version = Some(v); }
        if let Some(v) = msg.nav_modes { ac.nav_modes = Some(v); }
        ac.addr_type = msg.addr_type;

        // CPR position decode
        if let (Some(lat), Some(lon), Some(odd)) = (msg.cpr_lat, msg.cpr_lon, msg.cpr_odd) {
            if odd {
                ac.cpr_odd = Some((lat, lon, t));
            } else {
                ac.cpr_even = Some((lat, lon, t));
            }

            if let (Some((elat, elon, et)), Some((olat, olon, ot))) = (ac.cpr_even, ac.cpr_odd) {
                if (et - ot).abs() < 10.0 {
                    if let Some((lat, lon)) = cpr_global(elat, elon, olat, olon, ot > et) {
                        if lat.abs() <= 90.0 && lon.abs() <= 180.0 {
                            ac.lat = Some((lat * 1e6).round() / 1e6);
                            ac.lon = Some((lon * 1e6).round() / 1e6);
                            ac.seen_pos = Some(0.0);
                            ac.last_pos_update = t;

                            // Record trace point (max 1 per 4s, cap at 1000 points ~1hr)
                            let dominated = ac.trace.last().map(|p| t - p.ts < 4.0).unwrap_or(false);
                            if !dominated {
                                if ac.trace.len() >= 1000 { ac.trace.remove(0); }
                                ac.trace.push(TracePoint {
                                    ts: t, lat: ac.lat.unwrap(), lon: ac.lon.unwrap(),
                                    alt_baro: ac.alt_baro, alt_geom: ac.alt_geom,
                                    gs: ac.gs, track: ac.track,
                                    baro_rate: ac.baro_rate, ias: ac.ias,
                                });
                            }
                            if ac.source_type.is_none() {
                                ac.source_type = Some(if msg.df == 18 { "adsb_other".into() } else { "adsb".into() });
                            }
                        }
                    }
                }
            }
        }
    }

    /// Build the JSON cache (called periodically by json_builder)
    pub fn rebuild_json(&self) {
        let t = now();
        let total_msgs = self.messages_total.load(std::sync::atomic::Ordering::Relaxed);

        // Pre-allocate ~500 bytes per aircraft
        let count = self.map.len();
        let mut buf = Vec::with_capacity(count * 500 + 200);

        buf.extend_from_slice(b"{\"now\":");
        buf.extend_from_slice(format!("{:.1}", t).as_bytes());
        buf.extend_from_slice(b",\"messages\":");
        buf.extend_from_slice(total_msgs.to_string().as_bytes());
        buf.extend_from_slice(b",\"aircraft\":[");

        let mut first = true;
        for entry in self.map.iter() {
            let ac = entry.value();
            if !first { buf.push(b','); }
            first = false;

            // Manual JSON — faster than serde for hot path
            buf.push(b'{');
            write_str(&mut buf, "hex", &ac.hex);
            if let Some(ref f) = ac.flight { buf.push(b','); write_str(&mut buf, "flight", f); }
            if let Some(v) = ac.alt_baro { buf.push(b','); write_int(&mut buf, "alt_baro", v); }
            if let Some(v) = ac.alt_geom { buf.push(b','); write_int(&mut buf, "alt_geom", v); }
            if let Some(v) = ac.gs { buf.push(b','); write_float(&mut buf, "gs", v); }
            if let Some(v) = ac.track { buf.push(b','); write_float(&mut buf, "track", v); }
            if let Some(v) = ac.baro_rate { buf.push(b','); write_int(&mut buf, "baro_rate", v); }
            if let Some(v) = ac.geom_rate { buf.push(b','); write_int(&mut buf, "geom_rate", v); }
            if let Some(ref v) = ac.squawk { buf.push(b','); write_str(&mut buf, "squawk", v); }
            if let Some(ref v) = ac.category { buf.push(b','); write_str(&mut buf, "category", v); }
            if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
                if ac.last_pos_update > 0.0 && (t - ac.last_pos_update) < 60.0 {
                    buf.push(b','); write_float(&mut buf, "lat", lat);
                    buf.push(b','); write_float(&mut buf, "lon", lon);
                    if let Some(ref v) = ac.source_type { buf.push(b','); write_str(&mut buf, "type", v); }
                    buf.push(b','); write_float(&mut buf, "seen_pos", t - ac.last_pos_update);
                }
            }
            if let Some(v) = ac.ias { buf.push(b','); write_int(&mut buf, "ias", v as i32); }
            if let Some(v) = ac.tas { buf.push(b','); write_int(&mut buf, "tas", v as i32); }
            if let Some(v) = ac.mach { buf.push(b','); write_float(&mut buf, "mach", v); }
            if let Some(v) = ac.mag_heading { buf.push(b','); write_float(&mut buf, "mag_heading", v); }
            if let Some(v) = ac.true_heading { buf.push(b','); write_float(&mut buf, "true_heading", v); }
            if let Some(v) = ac.roll { buf.push(b','); write_float(&mut buf, "roll", v); }
            if let Some(v) = ac.track_rate { buf.push(b','); write_float(&mut buf, "track_rate", v); }
            if let Some(v) = ac.nav_altitude_mcp { buf.push(b','); write_int(&mut buf, "nav_altitude_mcp", v as i32); }
            if let Some(v) = ac.nav_altitude_fms { buf.push(b','); write_int(&mut buf, "nav_altitude_fms", v as i32); }
            if let Some(v) = ac.nav_qnh { buf.push(b','); write_float(&mut buf, "nav_qnh", v); }
            if let Some(v) = ac.nav_heading { buf.push(b','); write_float(&mut buf, "nav_heading", v); }
            if let Some(v) = ac.emergency { buf.push(b','); write_int(&mut buf, "emergency", v as i32); }
            if let Some(v) = ac.nic { buf.push(b','); write_int(&mut buf, "nic", v as i32); }
            if let Some(v) = ac.nac_p { buf.push(b','); write_int(&mut buf, "nac_p", v as i32); }
            if let Some(v) = ac.nac_v { buf.push(b','); write_int(&mut buf, "nac_v", v as i32); }
            if let Some(v) = ac.sil { buf.push(b','); write_int(&mut buf, "sil", v as i32); }
            if let Some(v) = ac.gva { buf.push(b','); write_int(&mut buf, "gva", v as i32); }
            if let Some(v) = ac.sda { buf.push(b','); write_int(&mut buf, "sda", v as i32); }
            if let Some(v) = ac.adsb_version { buf.push(b','); write_int(&mut buf, "version", v as i32); }
            buf.push(b','); write_float(&mut buf, "seen", t - ac.last_update);
            if let Some(v) = ac.rssi { buf.push(b','); write_float(&mut buf, "rssi", v); }
            buf.push(b','); write_u64(&mut buf, "messages", ac.messages);
            buf.push(b'}');
        }

        buf.extend_from_slice(b"]}");

        *self.json_cache.write() = bytes::Bytes::from(buf);
    }
}

/// Build JSON filtered by bounding box
pub fn build_json_filtered(store: &Store, south: f64, north: f64, west: f64, east: f64) -> Vec<u8> {
    let t = now();
    let total_msgs = store.messages_total.load(std::sync::atomic::Ordering::Relaxed);
    let mut buf = Vec::with_capacity(256 * 1024);
    buf.extend_from_slice(b"{\"now\":");
    buf.extend_from_slice(format!("{:.1}", t).as_bytes());
    buf.extend_from_slice(b",\"messages\":");
    buf.extend_from_slice(total_msgs.to_string().as_bytes());
    buf.extend_from_slice(b",\"aircraft\":[");
    let mut first = true;
    for entry in store.map.iter() {
        let ac = entry.value();
        if let (Some(lat), Some(lon)) = (ac.lat, ac.lon) {
            if lat >= south && lat <= north && crate::bincraft::lon_in_box(lon, west, east) {
                if !first { buf.push(b','); }
                first = false;
                // Reuse serde for filtered (not hot path)
                buf.extend_from_slice(&serde_json::to_vec(ac).unwrap_or_default());
            }
        }
    }
    buf.extend_from_slice(b"]}");
    buf
}

// Fast manual JSON writers — avoid serde overhead
fn write_str(buf: &mut Vec<u8>, key: &str, val: &str) {
    buf.push(b'"'); buf.extend_from_slice(key.as_bytes()); buf.extend_from_slice(b"\":\"");
    buf.extend_from_slice(val.as_bytes()); buf.push(b'"');
}
fn write_int(buf: &mut Vec<u8>, key: &str, val: i32) {
    buf.push(b'"'); buf.extend_from_slice(key.as_bytes()); buf.extend_from_slice(b"\":");
    buf.extend_from_slice(val.to_string().as_bytes());
}
fn write_u64(buf: &mut Vec<u8>, key: &str, val: u64) {
    buf.push(b'"'); buf.extend_from_slice(key.as_bytes()); buf.extend_from_slice(b"\":");
    buf.extend_from_slice(val.to_string().as_bytes());
}
fn write_float(buf: &mut Vec<u8>, key: &str, val: f64) {
    buf.push(b'"'); buf.extend_from_slice(key.as_bytes()); buf.extend_from_slice(b"\":");
    buf.extend_from_slice(format!("{:.6}", val).trim_end_matches('0').trim_end_matches('.').as_bytes());
}

// --- CPR global decode ---
fn cpr_global(elat: u32, elon: u32, olat: u32, olon: u32, odd_recent: bool) -> Option<(f64, f64)> {
    let lat0 = elat as f64 / 131072.0;
    let lat1 = olat as f64 / 131072.0;
    let lon0 = elon as f64 / 131072.0;
    let lon1 = olon as f64 / 131072.0;

    // Latitude
    let j = (59.0 * lat0 - 60.0 * lat1 + 0.5).floor() as i32;
    let mut rlat0 = (360.0 / 60.0) * (cpr_mod(j, 60) as f64 + lat0);
    let mut rlat1 = (360.0 / 59.0) * (cpr_mod(j, 59) as f64 + lat1);
    if rlat0 >= 270.0 { rlat0 -= 360.0; }
    if rlat1 >= 270.0 { rlat1 -= 360.0; }

    // Check NL consistency
    let nl0 = cpr_nl(rlat0);
    let nl1 = cpr_nl(rlat1);
    if nl0 != nl1 { return None; }

    // Longitude — ni depends on which frame is most recent
    let rlat = if odd_recent { rlat1 } else { rlat0 };
    let nl = cpr_nl(rlat);
    let ni = if odd_recent { (nl - 1).max(1) } else { nl.max(1) };
    let dlon = 360.0 / ni as f64;
    let m = (lon0 * (nl as f64 - 1.0) - lon1 * nl as f64 + 0.5).floor() as i32;
    let mut rlon = if odd_recent {
        dlon * (cpr_mod(m, ni) as f64 + lon1)
    } else {
        dlon * (cpr_mod(m, ni) as f64 + lon0)
    };
    if rlon >= 180.0 { rlon -= 360.0; }

    Some((rlat, rlon))
}

fn cpr_mod(a: i32, b: i32) -> i32 { ((a % b) + b) % b }

fn cpr_nl(lat: f64) -> i32 {
    let lat = lat.abs();
    if lat < 10.47047130  { return 59; }
    if lat < 14.82817437  { return 58; }
    if lat < 18.18626357  { return 57; }
    if lat < 21.02939493  { return 56; }
    if lat < 23.54504487  { return 55; }
    if lat < 25.82924707  { return 54; }
    if lat < 27.93898710  { return 53; }
    if lat < 29.91135686  { return 52; }
    if lat < 31.77209708  { return 51; }
    if lat < 33.53993436  { return 50; }
    if lat < 35.22899598  { return 49; }
    if lat < 36.85025108  { return 48; }
    if lat < 38.41241892  { return 47; }
    if lat < 39.92256684  { return 46; }
    if lat < 41.38651832  { return 45; }
    if lat < 42.80914012  { return 44; }
    if lat < 44.19454951  { return 43; }
    if lat < 45.54626723  { return 42; }
    if lat < 46.86733252  { return 41; }
    if lat < 48.16039128  { return 40; }
    if lat < 49.42776439  { return 39; }
    if lat < 50.67150166  { return 38; }
    if lat < 51.89342469  { return 37; }
    if lat < 53.09516153  { return 36; }
    if lat < 54.27817472  { return 35; }
    if lat < 55.44378444  { return 34; }
    if lat < 56.59318756  { return 33; }
    if lat < 57.72747354  { return 32; }
    if lat < 58.84763776  { return 31; }
    if lat < 59.95459277  { return 30; }
    if lat < 61.04917774  { return 29; }
    if lat < 62.13216659  { return 28; }
    if lat < 63.20427479  { return 27; }
    if lat < 64.26616523  { return 26; }
    if lat < 65.31845310  { return 25; }
    if lat < 66.36171008  { return 24; }
    if lat < 67.39646774  { return 23; }
    if lat < 68.42322022  { return 22; }
    if lat < 69.44242631  { return 21; }
    if lat < 70.45451075  { return 20; }
    if lat < 71.45986473  { return 19; }
    if lat < 72.45884545  { return 18; }
    if lat < 73.45177442  { return 17; }
    if lat < 74.43893416  { return 16; }
    if lat < 75.42056257  { return 15; }
    if lat < 76.39684391  { return 14; }
    if lat < 77.36789461  { return 13; }
    if lat < 78.33374083  { return 12; }
    if lat < 79.29428225  { return 11; }
    if lat < 80.24923213  { return 10; }
    if lat < 81.19801349  { return 9; }
    if lat < 82.13956981  { return 8; }
    if lat < 83.07199445  { return 7; }
    if lat < 83.99173563  { return 6; }
    if lat < 84.89166191  { return 5; }
    if lat < 85.75541621  { return 4; }
    if lat < 86.53536998  { return 3; }
    if lat < 87.00000000  { return 2; }
    1
}

// --- Reaper ---
pub async fn reaper(store: Arc<Store>) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let t = now();
        let before = store.map.len();
        store.map.retain(|_, ac| t - ac.last_update < 300.0);
        let removed = before - store.map.len();
        if removed > 0 {
            info!("reaper: -{} aircraft, {} remaining", removed, store.map.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cpr_nl() {
        assert_eq!(cpr_nl(0.0), 59);
        assert_eq!(cpr_nl(52.0), 36);
        assert_eq!(cpr_nl(87.0), 1);
    }
    #[test]
    fn test_cpr_global() {
        // From "The 1090MHz Riddle" by Junzi Sun
        let r = cpr_global(93000, 51372, 74158, 50194, false).unwrap();
        assert!((r.0 - 52.2572).abs() < 0.001, "lat={}", r.0);
        assert!((r.1 - 3.9192).abs() < 0.01, "lon={}", r.1);
    }
}
