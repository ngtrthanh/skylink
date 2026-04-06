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

    #[serde(skip)] pub cpr_even: Option<(u32, u32, f64)>,
    #[serde(skip)] pub cpr_odd: Option<(u32, u32, f64)>,
    #[serde(skip)] pub last_update: f64,
}

pub struct Store {
    pub map: DashMap<u32, Aircraft>,
    /// Pre-built JSON response — updated every ~1s by json_builder
    pub json_cache: RwLock<bytes::Bytes>,
    /// Pre-built binCraft response — updated every ~1s
    pub bincraft_cache: RwLock<bytes::Bytes>,
    /// Pre-built protobuf response
    pub pb_cache: RwLock<bytes::Bytes>,
    pub messages_total: std::sync::atomic::AtomicU64,
}

impl Store {
    pub fn new() -> Self {
        Self {
            map: DashMap::with_capacity(16384),
            json_cache: RwLock::new(bytes::Bytes::from_static(b"{\"now\":0,\"messages\":0,\"aircraft\":[]}")),
            bincraft_cache: RwLock::new(bytes::Bytes::new()),
            pb_cache: RwLock::new(bytes::Bytes::new()),
            messages_total: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn update_from_message(&self, msg: &Message, signal: u8) {
        if msg.icao == 0 { return; }
        let t = now();
        self.messages_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let mut entry = self.map.entry(msg.icao).or_insert_with(|| Aircraft {
            hex: format!("{:06x}", msg.icao),
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
            cpr_even: None, cpr_odd: None, last_update: t,
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
            if let Some(v) = ac.lat { buf.push(b','); write_float(&mut buf, "lat", v); }
            if let Some(v) = ac.lon { buf.push(b','); write_float(&mut buf, "lon", v); }
            if let Some(ref v) = ac.source_type { buf.push(b','); write_str(&mut buf, "type", v); }
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
            if ac.lat.is_some() { buf.push(b','); write_float(&mut buf, "seen_pos", t - ac.last_update); }
            if let Some(v) = ac.rssi { buf.push(b','); write_float(&mut buf, "rssi", v); }
            buf.push(b','); write_u64(&mut buf, "messages", ac.messages);
            buf.push(b'}');
        }

        buf.extend_from_slice(b"]}");

        *self.json_cache.write() = bytes::Bytes::from(buf);
    }
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

    let j = (59.0 * lat0 - 60.0 * lat1 + 0.5).floor() as i32;
    let mut rlat0 = (360.0 / 60.0) * (cpr_mod(j, 60) as f64 + lat0);
    let mut rlat1 = (360.0 / 59.0) * (cpr_mod(j, 59) as f64 + lat1);
    if rlat0 >= 270.0 { rlat0 -= 360.0; }
    if rlat1 >= 270.0 { rlat1 -= 360.0; }

    let nl0 = cpr_nl(rlat0);
    let nl1 = cpr_nl(rlat1);
    if nl0 != nl1 { return None; }

    let (rlat, nl) = if odd_recent { (rlat1, nl1) } else { (rlat0, nl0) };
    let ni = if nl > 0 { nl } else { 1 } as f64;
    let dlon = 360.0 / ni;
    let m = ((lon0 * (nl as f64 - 1.0) - lon1 * nl as f64 + 0.5).floor()) as i32;
    let mut rlon = if odd_recent {
        dlon * (cpr_mod(m, ni as i32) as f64 + lon1)
    } else {
        dlon * (cpr_mod(m, ni as i32) as f64 + lon0)
    };
    if rlon >= 180.0 { rlon -= 360.0; }

    Some((rlat, rlon))
}

fn cpr_mod(a: i32, b: i32) -> i32 { ((a % b) + b) % b }

fn cpr_nl(lat: f64) -> i32 {
    if lat.abs() >= 87.0 { return 1; }
    let nz = 15.0_f64;
    let a = 1.0 - (std::f64::consts::PI / (2.0 * nz)).cos();
    let b = (std::f64::consts::PI / 180.0 * lat).cos().powi(2);
    let nl = (2.0 * std::f64::consts::PI / (1.0 - (1.0 - a / b).acos())).floor() as i32;
    if nl < 1 { 1 } else { nl }
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
