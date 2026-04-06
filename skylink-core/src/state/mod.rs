pub mod reaper;

use dashmap::DashMap;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64()
}

#[derive(Debug, Clone, Serialize)]
pub struct Aircraft {
    pub hex: String,
    pub icao: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_baro: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_geom: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gs: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
    pub seen: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seen_pos: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi: Option<f64>,
    pub messages: u64,

    // CPR state (not serialized)
    #[serde(skip)]
    pub cpr_even_lat: Option<u32>,
    #[serde(skip)]
    pub cpr_even_lon: Option<u32>,
    #[serde(skip)]
    pub cpr_even_time: f64,
    #[serde(skip)]
    pub cpr_odd_lat: Option<u32>,
    #[serde(skip)]
    pub cpr_odd_lon: Option<u32>,
    #[serde(skip)]
    pub cpr_odd_time: f64,
    #[serde(skip)]
    pub last_update: f64,
}

impl Aircraft {
    pub fn new(icao: u32) -> Self {
        let now = now_secs();
        Self {
            hex: format!("{:06x}", icao),
            icao,
            flight: None,
            alt_baro: None,
            alt_geom: None,
            gs: None,
            track: None,
            lat: None,
            lon: None,
            seen: 0.0,
            seen_pos: None,
            rssi: None,
            messages: 0,
            cpr_even_lat: None,
            cpr_even_lon: None,
            cpr_even_time: 0.0,
            cpr_odd_lat: None,
            cpr_odd_lon: None,
            cpr_odd_time: 0.0,
            last_update: now,
        }
    }
}

pub struct AircraftUpdate {
    pub icao: u32,
    pub callsign: Option<String>,
    pub alt_baro: Option<i32>,
    pub alt_geom: Option<i32>,
    pub gs: Option<f64>,
    pub track: Option<f64>,
    pub signal: Option<u8>,
    pub cpr_lat: Option<u32>,
    pub cpr_lon: Option<u32>,
    pub cpr_odd: Option<bool>,
}

impl AircraftUpdate {
    pub fn new(icao: u32) -> Self {
        Self {
            icao, callsign: None, alt_baro: None, alt_geom: None,
            gs: None, track: None, signal: None,
            cpr_lat: None, cpr_lon: None, cpr_odd: None,
        }
    }
}

pub struct AircraftStore {
    pub map: DashMap<u32, Aircraft>,
}

impl AircraftStore {
    pub fn new() -> Self {
        Self { map: DashMap::with_capacity(16384) }
    }

    pub fn update(&self, u: AircraftUpdate) {
        let now = now_secs();
        let mut entry = self.map.entry(u.icao).or_insert_with(|| Aircraft::new(u.icao));
        let ac = entry.value_mut();

        ac.messages += 1;
        ac.last_update = now;

        if let Some(cs) = u.callsign { ac.flight = Some(cs); }
        if let Some(a) = u.alt_baro { ac.alt_baro = Some(a); }
        if let Some(a) = u.alt_geom { ac.alt_geom = Some(a); }
        if let Some(s) = u.gs { ac.gs = Some(s); }
        if let Some(t) = u.track { ac.track = Some(t); }
        if let Some(sig) = u.signal {
            ac.rssi = Some(10.0 * ((sig as f64 / 255.0).powi(2)).log10());
        }

        // CPR position decoding
        if let (Some(lat), Some(lon), Some(odd)) = (u.cpr_lat, u.cpr_lon, u.cpr_odd) {
            if odd {
                ac.cpr_odd_lat = Some(lat);
                ac.cpr_odd_lon = Some(lon);
                ac.cpr_odd_time = now;
            } else {
                ac.cpr_even_lat = Some(lat);
                ac.cpr_even_lon = Some(lon);
                ac.cpr_even_time = now;
            }

            // Try global CPR decode if we have both even and odd within 10 seconds
            if ac.cpr_even_lat.is_some() && ac.cpr_odd_lat.is_some()
                && (ac.cpr_even_time - ac.cpr_odd_time).abs() < 10.0
            {
                if let Some((lat, lon)) = cpr_global_decode(
                    ac.cpr_even_lat.unwrap(), ac.cpr_even_lon.unwrap(),
                    ac.cpr_odd_lat.unwrap(), ac.cpr_odd_lon.unwrap(),
                    ac.cpr_odd_time > ac.cpr_even_time,
                ) {
                    if lat.abs() <= 90.0 && lon.abs() <= 180.0 {
                        ac.lat = Some(lat);
                        ac.lon = Some(lon);
                        ac.seen_pos = Some(0.0);
                    }
                }
            }
        }
    }

    pub fn aircraft_json(&self, now: f64) -> serde_json::Value {
        let aircraft: Vec<serde_json::Value> = self.map.iter().map(|entry| {
            let ac = entry.value();
            let mut v = serde_json::to_value(ac).unwrap_or_default();
            if let Some(obj) = v.as_object_mut() {
                obj.insert("seen".to_string(), serde_json::json!(now - ac.last_update));
                if let Some(_) = ac.lat {
                    obj.insert("seen_pos".to_string(), serde_json::json!(now - ac.last_update));
                }
            }
            v
        }).collect();

        serde_json::json!({
            "now": now,
            "messages": self.map.iter().map(|e| e.value().messages).sum::<u64>(),
            "aircraft": aircraft,
        })
    }
}

// CPR global decode (airborne)
fn cpr_global_decode(even_lat: u32, even_lon: u32, odd_lat: u32, odd_lon: u32, odd_recent: bool) -> Option<(f64, f64)> {
    let air_dlat0 = 360.0 / 60.0;
    let air_dlat1 = 360.0 / 59.0;

    let lat0 = even_lat as f64 / 131072.0;
    let lat1 = odd_lat as f64 / 131072.0;
    let lon0 = even_lon as f64 / 131072.0;
    let lon1 = odd_lon as f64 / 131072.0;

    let j = ((59.0 * lat0 - 60.0 * lat1 + 0.5).floor()) as i32;

    let mut rlat0 = air_dlat0 * (cpr_mod(j, 60) as f64 + lat0);
    let mut rlat1 = air_dlat1 * (cpr_mod(j, 59) as f64 + lat1);

    if rlat0 >= 270.0 { rlat0 -= 360.0; }
    if rlat1 >= 270.0 { rlat1 -= 360.0; }

    let nl0 = cpr_nl(rlat0);
    let nl1 = cpr_nl(rlat1);
    if nl0 != nl1 { return None; } // straddling NL boundary

    let (rlat, nl) = if odd_recent { (rlat1, nl1) } else { (rlat0, nl0) };

    let ni = if nl > 0 { nl } else { 1 } as f64;
    let dlon = 360.0 / ni;
    let m = ((lon0 * (nl as f64 - 1.0) - lon1 * nl as f64 + 0.5).floor()) as i32;

    let rlon = if odd_recent {
        dlon * (cpr_mod(m, ni as i32) as f64 + lon1)
    } else {
        dlon * (cpr_mod(m, ni as i32) as f64 + lon0)
    };

    let rlon = if rlon >= 180.0 { rlon - 360.0 } else { rlon };

    Some((rlat, rlon))
}

fn cpr_mod(a: i32, b: i32) -> i32 {
    ((a % b) + b) % b
}

fn cpr_nl(lat: f64) -> i32 {
    if lat.abs() >= 87.0 { return 1; }
    let tmp = 1.0 - (std::f64::consts::PI / 30.0).cos();
    let nz = 15.0;
    let val = (1.0 - tmp / ((std::f64::consts::PI / (2.0 * nz) * lat.to_radians().cos().acos()).cos().powi(2))).acos() / (2.0 * std::f64::consts::PI);
    let nl = val.floor() as i32;
    if nl < 1 { 1 } else { nl }
}
