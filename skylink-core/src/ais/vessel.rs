/// Vessel data model and store
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Default)]
pub struct VesselUpdate {
    pub mmsi: u32,
    pub msg_type: u8,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub speed: Option<f32>,
    pub cog: Option<f32>,
    pub heading: Option<u16>,
    pub status: Option<u8>,
    pub turn: Option<i16>,
    pub shiptype: Option<u8>,
    pub shipclass: u8,
    pub shipname: Option<String>,
    pub callsign: Option<String>,
    pub destination: Option<String>,
    pub imo: Option<u32>,
    pub draught: Option<f32>,
    pub to_bow: Option<u16>,
    pub to_stern: Option<u16>,
    pub to_port: Option<u16>,
    pub to_starboard: Option<u16>,
    pub eta_month: Option<u8>,
    pub eta_day: Option<u8>,
    pub eta_hour: Option<u8>,
    pub eta_minute: Option<u8>,
}

#[derive(Clone)]
pub struct Vessel {
    pub mmsi: u32,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub speed: Option<f32>,
    pub cog: Option<f32>,
    pub heading: Option<u16>,
    pub status: Option<u8>,
    pub turn: Option<i16>,
    pub shiptype: u8,
    pub shipclass: u8,
    pub shipname: String,
    pub callsign: String,
    pub destination: String,
    pub imo: Option<u32>,
    pub draught: Option<f32>,
    pub to_bow: Option<u16>,
    pub to_stern: Option<u16>,
    pub to_port: Option<u16>,
    pub to_starboard: Option<u16>,
    pub eta_month: Option<u8>,
    pub eta_day: Option<u8>,
    pub eta_hour: Option<u8>,
    pub eta_minute: Option<u8>,
    pub country: String,
    pub count: u32,
    pub last_signal: f64,
    pub last_pos_update: f64,
}

impl Vessel {
    fn new(mmsi: u32) -> Self {
        Self {
            mmsi, country: mmsi_country(mmsi),
            lat: None, lon: None, speed: None, cog: None, heading: None,
            status: None, turn: None, shiptype: 0, shipclass: 0,
            shipname: String::new(), callsign: String::new(), destination: String::new(),
            imo: None, draught: None,
            to_bow: None, to_stern: None, to_port: None, to_starboard: None,
            eta_month: None, eta_day: None, eta_hour: None, eta_minute: None,
            count: 0, last_signal: 0.0, last_pos_update: 0.0,
        }
    }

    pub fn apply(&mut self, u: &VesselUpdate) {
        let now = now_secs();
        self.count += 1;
        self.last_signal = now;
        if u.shipclass != 0 { self.shipclass = u.shipclass; }
        if let Some(v) = u.lat { self.lat = Some(v); self.last_pos_update = now; }
        if let Some(v) = u.lon { self.lon = Some(v); }
        if let Some(v) = u.speed { self.speed = Some(v); }
        if let Some(v) = u.cog { self.cog = Some(v); }
        if let Some(v) = u.heading { self.heading = Some(v); }
        if let Some(v) = u.status { self.status = Some(v); }
        if let Some(v) = u.turn { self.turn = Some(v); }
        if let Some(v) = u.shiptype { if v != 0 { self.shiptype = v; } }
        if let Some(ref v) = u.shipname { if !v.is_empty() { self.shipname = v.clone(); } }
        if let Some(ref v) = u.callsign { if !v.is_empty() { self.callsign = v.clone(); } }
        if let Some(ref v) = u.destination { if !v.is_empty() { self.destination = v.clone(); } }
        if let Some(v) = u.imo { self.imo = Some(v); }
        if let Some(v) = u.draught { self.draught = Some(v); }
        if let Some(v) = u.to_bow { self.to_bow = Some(v); }
        if let Some(v) = u.to_stern { self.to_stern = Some(v); }
        if let Some(v) = u.to_port { self.to_port = Some(v); }
        if let Some(v) = u.to_starboard { self.to_starboard = Some(v); }
        if let Some(v) = u.eta_month { self.eta_month = Some(v); }
        if let Some(v) = u.eta_day { self.eta_day = Some(v); }
        if let Some(v) = u.eta_hour { self.eta_hour = Some(v); }
        if let Some(v) = u.eta_minute { self.eta_minute = Some(v); }
    }
}

pub struct VesselStore {
    pub map: DashMap<u32, Vessel>,
    pub messages_total: AtomicU64,
    pub json_cache: parking_lot::RwLock<bytes::Bytes>,
    pub geojson_cache: parking_lot::RwLock<bytes::Bytes>,
}

impl VesselStore {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            messages_total: AtomicU64::new(0),
            json_cache: parking_lot::RwLock::new(bytes::Bytes::new()),
            geojson_cache: parking_lot::RwLock::new(bytes::Bytes::new()),
        }
    }

    pub fn update(&self, u: VesselUpdate) {
        self.messages_total.fetch_add(1, Ordering::Relaxed);
        self.map.entry(u.mmsi).or_insert_with(|| Vessel::new(u.mmsi)).apply(&u);
    }

    pub fn rebuild_caches(&self) {
        *self.json_cache.write() = bytes::Bytes::from(self.build_json());
        *self.geojson_cache.write() = bytes::Bytes::from(self.build_geojson());
    }

    fn build_json(&self) -> Vec<u8> {
        let now = now_secs();
        let mut out = String::with_capacity(self.map.len() * 256);
        out.push_str("{\"vessels\":[");
        let mut first = true;
        for entry in self.map.iter() {
            let v = entry.value();
            if now - v.last_signal > 600.0 { continue; } // 10min timeout
            if !first { out.push(','); }
            first = false;
            vessel_json(v, &mut out, now);
        }
        out.push_str("]}");
        out.into_bytes()
    }

    fn build_geojson(&self) -> Vec<u8> {
        let now = now_secs();
        let mut out = String::with_capacity(self.map.len() * 300);
        out.push_str("{\"type\":\"FeatureCollection\",\"features\":[");
        let mut first = true;
        for entry in self.map.iter() {
            let v = entry.value();
            if now - v.last_signal > 600.0 { continue; }
            let (lat, lon) = match (v.lat, v.lon) { (Some(a), Some(b)) => (a, b), _ => continue };
            if !first { out.push(','); }
            first = false;
            out.push_str(&format!(
                "{{\"type\":\"Feature\",\"geometry\":{{\"type\":\"Point\",\"coordinates\":[{lon},{lat}]}},\"properties\":{{"));
            out.push_str(&format!("\"mmsi\":{},\"shipname\":\"{}\",\"callsign\":\"{}\",\"shiptype\":{},\"shipclass\":{},\"country\":\"{}\"",
                v.mmsi, esc(&v.shipname), esc(&v.callsign), v.shiptype, v.shipclass, v.country));
            if let Some(s) = v.speed { out.push_str(&format!(",\"speed\":{s:.1}")); }
            if let Some(c) = v.cog { out.push_str(&format!(",\"cog\":{c:.1}")); }
            if let Some(h) = v.heading { out.push_str(&format!(",\"heading\":{h}")); }
            if let Some(s) = v.status { out.push_str(&format!(",\"status\":{s}")); }
            if !v.destination.is_empty() { out.push_str(&format!(",\"destination\":\"{}\"", esc(&v.destination))); }
            if let Some(i) = v.imo { out.push_str(&format!(",\"imo\":{i}")); }
            if let Some(d) = v.draught { out.push_str(&format!(",\"draught\":{d:.1}")); }
            out.push_str(&format!(",\"count\":{},\"last_signal\":{:.0}", v.count, now - v.last_signal));
            out.push_str("}}");
        }
        out.push_str("]}");
        out.into_bytes()
    }

    pub fn build_geojson_filtered(&self, south: f64, north: f64, west: f64, east: f64) -> Vec<u8> {
        let now = now_secs();
        let mut out = String::with_capacity(4096);
        out.push_str("{\"type\":\"FeatureCollection\",\"features\":[");
        let mut first = true;
        for entry in self.map.iter() {
            let v = entry.value();
            if now - v.last_signal > 600.0 { continue; }
            let (lat, lon) = match (v.lat, v.lon) { (Some(a), Some(b)) => (a as f64, b as f64), _ => continue };
            if lat < south || lat > north { continue; }
            let lon_ok = if west <= east { lon >= west && lon <= east } else { lon >= west || lon <= east };
            if !lon_ok { continue; }
            if !first { out.push(','); }
            first = false;
            out.push_str(&format!(
                "{{\"type\":\"Feature\",\"geometry\":{{\"type\":\"Point\",\"coordinates\":[{lon},{lat}]}},\"properties\":{{"));
            out.push_str(&format!("\"mmsi\":{},\"shipname\":\"{}\",\"shiptype\":{},\"shipclass\":{}",
                v.mmsi, esc(&v.shipname), v.shiptype, v.shipclass));
            if let Some(s) = v.speed { out.push_str(&format!(",\"speed\":{s:.1}")); }
            if let Some(c) = v.cog { out.push_str(&format!(",\"cog\":{c:.1}")); }
            if let Some(h) = v.heading { out.push_str(&format!(",\"heading\":{h}")); }
            if let Some(s) = v.status { out.push_str(&format!(",\"status\":{s}")); }
            out.push_str(&format!(",\"country\":\"{}\"", v.country));
            out.push_str("}}");
        }
        out.push_str("]}");
        out.into_bytes()
    }
}

/// Reap stale vessels (>30min no signal)
pub async fn reaper(store: Arc<VesselStore>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let now = now_secs();
        store.map.retain(|_, v| now - v.last_signal < 1800.0);
    }
}

/// Cache rebuild loop (1s)
pub async fn cache_loop(store: Arc<VesselStore>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        interval.tick().await;
        store.rebuild_caches();
    }
}

pub fn vessel_json_pub(v: &Vessel, out: &mut String, now: f64) { vessel_json(v, out, now); }

fn vessel_json(v: &Vessel, out: &mut String, now: f64) {
    out.push_str(&format!("{{\"mmsi\":{}", v.mmsi));
    if let Some(lat) = v.lat { out.push_str(&format!(",\"lat\":{lat}")); }
    if let Some(lon) = v.lon { out.push_str(&format!(",\"lon\":{lon}")); }
    if let Some(s) = v.speed { out.push_str(&format!(",\"speed\":{s:.1}")); }
    if let Some(c) = v.cog { out.push_str(&format!(",\"cog\":{c:.1}")); }
    if let Some(h) = v.heading { out.push_str(&format!(",\"heading\":{h}")); }
    if let Some(s) = v.status { out.push_str(&format!(",\"status\":{s}")); }
    if !v.shipname.is_empty() { out.push_str(&format!(",\"shipname\":\"{}\"", esc(&v.shipname))); }
    if !v.callsign.is_empty() { out.push_str(&format!(",\"callsign\":\"{}\"", esc(&v.callsign))); }
    if !v.destination.is_empty() { out.push_str(&format!(",\"destination\":\"{}\"", esc(&v.destination))); }
    out.push_str(&format!(",\"shiptype\":{},\"shipclass\":{},\"country\":\"{}\"", v.shiptype, v.shipclass, v.country));
    if let Some(i) = v.imo { out.push_str(&format!(",\"imo\":{i}")); }
    if let Some(d) = v.draught { out.push_str(&format!(",\"draught\":{d:.1}")); }
    out.push_str(&format!(",\"count\":{},\"last_signal\":{:.0}}}", v.count, now - v.last_signal));
}

fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }

fn now_secs() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()
}

fn mmsi_country(mmsi: u32) -> String {
    let mid = (mmsi / 1000000) % 1000;
    match mid {
        201..=212 => "GR", 213..=214 => "TR", 215 => "MT", 216 => "CY", 218 => "DE",
        219 => "DK", 220 => "DK", 224..=227 => "ES", 228..=229 => "FR", 230 => "FI",
        231 => "FO", 232..=235 => "GB", 236 => "GI", 237 => "GR", 238 => "HR",
        239 => "GR", 240 => "GR", 241 => "GR", 242 => "MA", 243 => "HU",
        244..=246 => "NL", 247..=249 => "IT", 250 => "IE", 251 => "IS",
        255 => "PT", 256 => "MT", 257 => "NO", 258 => "NO", 259 => "NO",
        261 => "PL", 263 => "PT", 265..=266 => "SE", 267 => "CZ",
        268 => "UA", 269 => "RU", 270 => "CZ", 271 => "TR", 272 => "UA",
        273 => "RU", 274 => "IS", 275 => "LV", 276 => "EE", 277 => "LT",
        278 => "SI", 279 => "RS", 301 => "AI", 303 => "US", 304 => "AG",
        305 => "AG", 306 => "CW", 307 => "AW", 308 => "BS", 309 => "BS",
        310 => "BM", 311 => "BS", 312 => "BZ", 314 => "BB", 316 => "CA",
        319 => "KY", 321 => "CR", 323 => "CU", 325 => "DM", 327 => "DO",
        329 => "GP", 330 => "GD", 332 => "GT", 334 => "HN", 336 => "HT",
        338 => "US", 339 => "JM", 341 => "KN", 343 => "LC", 345 => "MX",
        347 => "MQ", 348 => "MS", 350 => "NI", 351..=354 => "PA",
        355..=357 => "PR", 358 => "PR", 361 => "PM", 362 => "TT",
        364 => "TC", 366..=369 => "US", 370..=372 => "PA", 373 => "PA",
        375 => "VC", 376 => "VC", 377 => "VC", 378 => "VG",
        379 => "VI", 401 => "AF", 403 => "SA", 405 => "BD",
        408 => "BH", 410 => "BT", 412..=413 => "CN", 414 => "CN",
        416 => "TW", 417 => "LK", 419 => "IN", 422 => "IR",
        423 => "AZ", 425 => "IQ", 428 => "IL", 431..=432 => "JP",
        434 => "TM", 436 => "KZ", 437 => "UZ", 438 => "JO",
        440..=441 => "KR", 443 => "PS", 445 => "KP", 447 => "KW",
        450 => "LB", 451 => "KG", 453 => "MO", 455 => "MV",
        457 => "MN", 459 => "NP", 461 => "OM", 463 => "PK",
        466 => "QA", 468 => "SY", 470 => "AE", 472 => "TJ",
        473 => "YE", 475 => "AF", 477 => "HK", 478 => "BA",
        501 => "AQ", 503 => "AU", 506 => "MM", 508 => "BN",
        510 => "FM", 511 => "PW", 512 => "NZ", 514 => "KH",
        515 => "KH", 516 => "CX", 518 => "CK", 520 => "FJ",
        523 => "CC", 525 => "ID", 529 => "KI", 531 => "LA",
        533 => "MY", 536 => "MP", 538 => "MH", 540 => "NC",
        542 => "NU", 544 => "NR", 546 => "PF", 548 => "PH",
        553 => "PG", 555 => "PN", 557 => "SB", 559 => "AS",
        561 => "WS", 563 => "SG", 564 => "SG", 565 => "SG",
        566 => "SG", 567 => "TH", 570 => "TO", 572 => "TV",
        574 => "VN", 576 => "VU", 577 => "VU", 578 => "WF",
        601 => "ZA", 603 => "AO", 605 => "DZ", 607 => "TF",
        608 => "IO", 609 => "BI", 610 => "BJ", 611 => "BW",
        612 => "CF", 613 => "CM", 615 => "CG", 616 => "KM",
        617 => "CV", 618 => "AQ", 619 => "CI", 620 => "KM",
        621 => "DJ", 622 => "EG", 624 => "ET", 625 => "ER",
        626 => "GA", 627 => "GH", 629 => "GM", 630 => "GW",
        631 => "GQ", 632 => "GN", 633 => "BF", 634 => "KE",
        635 => "AQ", 636 => "LR", 637 => "LR", 638 => "SS",
        642 => "LY", 644 => "LS", 645 => "MU", 647 => "MG",
        649 => "ML", 650 => "MZ", 654 => "MR", 655 => "MW",
        656 => "NE", 657 => "NG", 659 => "NA", 660 => "RE",
        661 => "RW", 662 => "SD", 663 => "SN", 664 => "SC",
        665 => "SH", 666 => "SO", 667 => "SL", 668 => "ST",
        669 => "SZ", 670 => "TD", 671 => "TG", 672 => "TN",
        674 => "TZ", 675 => "UG", 676 => "CD", 677 => "TZ",
        678 => "ZM", 679 => "ZW",
        _ => "??",
    }.to_string()
}
