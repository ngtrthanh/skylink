/// Vessel data model, store, paths, classification, and statistics
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

// --- Path ring buffer ---
const MAX_PATH_POINTS: usize = 256;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct PathPoint {
    pub lat: f32,
    pub lon: f32,
    pub ts: f64,
    pub speed: f32,
    pub cog: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PathBuffer {
    buf: Vec<PathPoint>,
    head: usize,
    len: usize,
}

impl PathBuffer {
    fn new() -> Self { Self { buf: vec![PathPoint { lat: 0.0, lon: 0.0, ts: 0.0, speed: 0.0, cog: 0.0 }; MAX_PATH_POINTS], head: 0, len: 0 } }
    fn push(&mut self, p: PathPoint) {
        self.buf[self.head] = p;
        self.head = (self.head + 1) % MAX_PATH_POINTS;
        if self.len < MAX_PATH_POINTS { self.len += 1; }
    }
    pub fn iter(&self) -> impl Iterator<Item = &PathPoint> {
        let start = if self.len < MAX_PATH_POINTS { 0 } else { self.head };
        let buf = &self.buf;
        let len = self.len;
        (0..len).map(move |i| &buf[(start + i) % MAX_PATH_POINTS])
    }
}

// --- Ship type classification ---
pub fn ship_type_class(shiptype: u8) -> &'static str {
    match shiptype {
        20..=29 => "WIG",
        30 => "Fishing",
        31..=32 => "Towing",
        33 => "Dredging",
        34 => "Diving",
        35 => "Military",
        36 => "Sailing",
        37 => "Pleasure",
        40..=49 => "HSC",
        50 => "Pilot",
        51 => "SAR",
        52 => "Tug",
        53 => "Port Tender",
        54 => "Anti-Pollution",
        55 => "Law Enforcement",
        58 => "Medical",
        60..=69 => "Passenger",
        70..=79 => "Cargo",
        80..=89 => "Tanker",
        90..=99 => "Other",
        _ => "Unknown",
    }
}

pub fn ship_class_name(shipclass: u8) -> &'static str {
    match shipclass {
        1 => "A",
        2 => "B",
        3 => "Base Station",
        4 => "SAR Aircraft",
        5 => "ATON",
        _ => "Unknown",
    }
}

// --- Update struct ---
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
    pub altitude: Option<u16>,
    pub text: Option<String>,
    pub epfd: Option<u8>,
    pub ais_version: Option<u8>,
    pub dte: Option<u8>,
    pub maneuver: Option<u8>,
    pub raim: Option<bool>,
    pub virtual_aid: Option<bool>,
    pub off_position: Option<bool>,
    pub aid_type: Option<u8>,
    pub mothership_mmsi: Option<u32>,
}

// --- Vessel ---
#[derive(Clone, serde::Serialize, serde::Deserialize)]
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
    pub altitude: Option<u16>,
    pub epfd: Option<u8>,
    pub ais_version: Option<u8>,
    pub dte: Option<u8>,
    pub maneuver: Option<u8>,
    pub raim: Option<bool>,
    pub virtual_aid: Option<bool>,
    pub off_position: Option<bool>,
    pub aid_type: Option<u8>,
    pub mothership_mmsi: Option<u32>,
    pub country: String,
    pub count: u32,
    pub last_signal: f64,
    pub last_pos_update: f64,
    pub path: PathBuffer,
}

impl Vessel {
    fn new(mmsi: u32) -> Self {
        Self {
            mmsi, country: mmsi_country(mmsi),
            lat: None, lon: None, speed: None, cog: None, heading: None,
            status: None, turn: None, shiptype: 0, shipclass: 0,
            shipname: String::new(), callsign: String::new(), destination: String::new(),
            imo: None, draught: None, altitude: None,
            epfd: None, ais_version: None, dte: None, maneuver: None,
            raim: None, virtual_aid: None, off_position: None,
            aid_type: None, mothership_mmsi: None,
            to_bow: None, to_stern: None, to_port: None, to_starboard: None,
            eta_month: None, eta_day: None, eta_hour: None, eta_minute: None,
            count: 0, last_signal: 0.0, last_pos_update: 0.0,
            path: PathBuffer::new(),
        }
    }

    pub fn apply(&mut self, u: &VesselUpdate) {
        let now = now_secs();
        self.count += 1;
        self.last_signal = now;
        if u.shipclass != 0 { self.shipclass = u.shipclass; }
        if let (Some(lat), Some(lon)) = (u.lat, u.lon) {
            // Add to path if moved enough or enough time passed
            let should_add = match (self.lat, self.lon) {
                (Some(olat), Some(olon)) => {
                    let dt = now - self.last_pos_update;
                    let dlat = (lat - olat).abs();
                    let dlon = (lon - olon).abs();
                    dt >= 4.0 || dlat > 0.0005 || dlon > 0.0005
                }
                _ => true,
            };
            self.lat = Some(lat);
            self.lon = Some(lon);
            if should_add {
                self.last_pos_update = now;
                self.path.push(PathPoint {
                    lat, lon, ts: now,
                    speed: u.speed.unwrap_or(self.speed.unwrap_or(0.0)),
                    cog: u.cog.unwrap_or(self.cog.unwrap_or(0.0)),
                });
            }
        }
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
        if let Some(v) = u.altitude { self.altitude = Some(v); }
        if let Some(v) = u.to_bow { self.to_bow = Some(v); }
        if let Some(v) = u.to_stern { self.to_stern = Some(v); }
        if let Some(v) = u.to_port { self.to_port = Some(v); }
        if let Some(v) = u.to_starboard { self.to_starboard = Some(v); }
        if let Some(v) = u.eta_month { self.eta_month = Some(v); }
        if let Some(v) = u.eta_day { self.eta_day = Some(v); }
        if let Some(v) = u.eta_hour { self.eta_hour = Some(v); }
        if let Some(v) = u.eta_minute { self.eta_minute = Some(v); }
        if let Some(v) = u.epfd { self.epfd = Some(v); }
        if let Some(v) = u.ais_version { self.ais_version = Some(v); }
        if let Some(v) = u.dte { self.dte = Some(v); }
        if let Some(v) = u.maneuver { self.maneuver = Some(v); }
        if let Some(v) = u.raim { self.raim = Some(v); }
        if let Some(v) = u.virtual_aid { self.virtual_aid = Some(v); }
        if let Some(v) = u.off_position { self.off_position = Some(v); }
        if let Some(v) = u.aid_type { self.aid_type = Some(v); }
        if let Some(v) = u.mothership_mmsi { self.mothership_mmsi = Some(v); }
    }

    pub fn type_class(&self) -> &'static str { ship_type_class(self.shiptype) }
    pub fn class_name(&self) -> &'static str { ship_class_name(self.shipclass) }
}

// --- Statistics ---
pub struct AisStats {
    pub msg_counts: [AtomicU64; 28], // per message type 0-27
    pub class_a: AtomicU64,
    pub class_b: AtomicU64,
    pub base_station: AtomicU64,
    pub aton: AtomicU64,
    pub sar: AtomicU64,
}

impl AisStats {
    fn new() -> Self {
        Self {
            msg_counts: std::array::from_fn(|_| AtomicU64::new(0)),
            class_a: AtomicU64::new(0),
            class_b: AtomicU64::new(0),
            base_station: AtomicU64::new(0),
            aton: AtomicU64::new(0),
            sar: AtomicU64::new(0),
        }
    }
    pub fn record(&self, msg_type: u8, shipclass: u8) {
        if (msg_type as usize) < 28 { self.msg_counts[msg_type as usize].fetch_add(1, Ordering::Relaxed); }
        match shipclass {
            1 => { self.class_a.fetch_add(1, Ordering::Relaxed); }
            2 => { self.class_b.fetch_add(1, Ordering::Relaxed); }
            3 => { self.base_station.fetch_add(1, Ordering::Relaxed); }
            4 => { self.sar.fetch_add(1, Ordering::Relaxed); }
            5 => { self.aton.fetch_add(1, Ordering::Relaxed); }
            _ => {}
        }
    }
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"msg_types\":{");
        let mut first = true;
        for i in 0..28 {
            let c = self.msg_counts[i].load(Ordering::Relaxed);
            if c > 0 {
                if !first { out.push(','); }
                first = false;
                out.push_str(&format!("\"{}\":{}", i, c));
            }
        }
        out.push_str(&format!("}},\"class_a\":{},\"class_b\":{},\"base_station\":{},\"aton\":{},\"sar\":{}}}",
            self.class_a.load(Ordering::Relaxed), self.class_b.load(Ordering::Relaxed),
            self.base_station.load(Ordering::Relaxed), self.aton.load(Ordering::Relaxed),
            self.sar.load(Ordering::Relaxed)));
        out
    }
}

// --- Store ---
pub struct VesselStore {
    pub map: DashMap<u32, Vessel>,
    pub messages_total: AtomicU64,
    pub stats: AisStats,
    pub json_cache: parking_lot::RwLock<bytes::Bytes>,
    pub geojson_cache: parking_lot::RwLock<bytes::Bytes>,
    pub binvessel_cache: parking_lot::RwLock<bytes::Bytes>,
    pub binvessel_zstd_cache: parking_lot::RwLock<bytes::Bytes>,
}

impl VesselStore {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            messages_total: AtomicU64::new(0),
            stats: AisStats::new(),
            json_cache: parking_lot::RwLock::new(bytes::Bytes::new()),
            geojson_cache: parking_lot::RwLock::new(bytes::Bytes::new()),
            binvessel_cache: parking_lot::RwLock::new(bytes::Bytes::new()),
            binvessel_zstd_cache: parking_lot::RwLock::new(bytes::Bytes::new()),
        }
    }

    pub fn update(&self, u: VesselUpdate) {
        self.messages_total.fetch_add(1, Ordering::Relaxed);
        self.stats.record(u.msg_type, u.shipclass);
        self.map.entry(u.mmsi).or_insert_with(|| Vessel::new(u.mmsi)).apply(&u);
    }

    /// Save vessel store to disk
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let vessels: Vec<Vessel> = self.map.iter().map(|e| e.value().clone()).collect();
        let json = serde_json::to_vec(&vessels).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let compressed = zstd_compress(&json);
        std::fs::write(path, &compressed)?;
        tracing::info!("ais: saved {} vessels to {path} ({} bytes)", vessels.len(), compressed.len());
        Ok(())
    }

    /// Load vessel store from disk
    pub fn load(&self, path: &str) -> std::io::Result<usize> {
        let compressed = std::fs::read(path)?;
        let json = zstd::decode_all(compressed.as_slice())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let vessels: Vec<Vessel> = serde_json::from_slice(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let count = vessels.len();
        for v in vessels { self.map.insert(v.mmsi, v); }
        tracing::info!("ais: loaded {count} vessels from {path}");
        Ok(count)
    }

    pub fn rebuild_caches(&self, store_arc: &Arc<VesselStore>) {
        *self.json_cache.write() = bytes::Bytes::from(self.build_json());
        *self.geojson_cache.write() = bytes::Bytes::from(self.build_geojson());
        let bin = crate::binvessel::build(store_arc);
        let zstd = zstd_compress(&bin);
        *self.binvessel_cache.write() = bytes::Bytes::from(bin);
        *self.binvessel_zstd_cache.write() = bytes::Bytes::from(zstd);
    }

    fn build_json(&self) -> Vec<u8> {
        let now = now_secs();
        let mut out = String::with_capacity(self.map.len() * 300);
        out.push_str("{\"vessels\":[");
        let mut first = true;
        for entry in self.map.iter() {
            let v = entry.value();
            if now - v.last_signal > 1800.0 { continue; }
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
            if now - v.last_signal > 1800.0 { continue; }
            let (lat, lon) = match (v.lat, v.lon) { (Some(a), Some(b)) => (a, b), _ => continue };
            if !first { out.push(','); }
            first = false;
            write_geojson_feature(v, lat, lon, &mut out, now);
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
            if now - v.last_signal > 1800.0 { continue; }
            let (lat, lon) = match (v.lat, v.lon) { (Some(a), Some(b)) => (a as f64, b as f64), _ => continue };
            if lat < south || lat > north { continue; }
            let lon_ok = if west <= east { lon >= west && lon <= east } else { lon >= west || lon <= east };
            if !lon_ok { continue; }
            if !first { out.push(','); }
            first = false;
            write_geojson_feature(v, lat as f32, lon as f32, &mut out, now);
        }
        out.push_str("]}");
        out.into_bytes()
    }

    pub fn get_path_json(&self, mmsi: u32) -> Option<String> {
        let entry = self.map.get(&mmsi)?;
        let v = entry.value();
        let mut out = String::with_capacity(v.path.len * 64);
        out.push_str(&format!("{{\"mmsi\":{},\"path\":[", mmsi));
        let mut first = true;
        for p in v.path.iter() {
            if !first { out.push(','); }
            first = false;
            out.push_str(&format!("[{},{},{:.0},{:.1},{:.1}]", p.lat, p.lon, p.ts, p.speed, p.cog));
        }
        out.push_str("]}");
        Some(out)
    }

    pub fn get_path_geojson(&self, mmsi: u32) -> Option<String> {
        let entry = self.map.get(&mmsi)?;
        let v = entry.value();
        if v.path.len < 2 { return None; }
        let mut coords = String::new();
        let mut first = true;
        for p in v.path.iter() {
            if !first { coords.push(','); }
            first = false;
            coords.push_str(&format!("[{},{}]", p.lon, p.lat));
        }
        Some(format!("{{\"type\":\"Feature\",\"geometry\":{{\"type\":\"LineString\",\"coordinates\":[{coords}]}},\"properties\":{{\"mmsi\":{},\"shipname\":\"{}\"}}}}",
            mmsi, esc(&v.shipname)))
    }

    pub fn get_all_paths_geojson(&self) -> Vec<u8> {
        let now = now_secs();
        let mut out = String::with_capacity(8192);
        out.push_str("{\"type\":\"FeatureCollection\",\"features\":[");
        let mut first = true;
        for entry in self.map.iter() {
            let v = entry.value();
            if now - v.last_signal > 1800.0 || v.path.len < 2 { continue; }
            if !first { out.push(','); }
            first = false;
            out.push_str(&format!("{{\"type\":\"Feature\",\"geometry\":{{\"type\":\"LineString\",\"coordinates\":["));
            let mut fp = true;
            for p in v.path.iter() {
                if !fp { out.push(','); }
                fp = false;
                out.push_str(&format!("[{},{}]", p.lon, p.lat));
            }
            out.push_str(&format!("]}},\"properties\":{{\"mmsi\":{},\"shipname\":\"{}\",\"shiptype\":{},\"type_class\":\"{}\"}}}}",
                v.mmsi, esc(&v.shipname), v.shiptype, v.type_class()));
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
    loop { interval.tick().await; store.rebuild_caches(&store); }
}

fn zstd_compress(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut out = Vec::with_capacity(data.len() / 2);
    let mut enc = zstd::Encoder::new(&mut out, 3).unwrap();
    enc.set_pledged_src_size(Some(data.len() as u64)).ok();
    enc.write_all(data).unwrap();
    enc.finish().unwrap();
    out
}

// --- JSON helpers ---

fn write_geojson_feature(v: &Vessel, lat: f32, lon: f32, out: &mut String, now: f64) {
    out.push_str(&format!(
        "{{\"type\":\"Feature\",\"geometry\":{{\"type\":\"Point\",\"coordinates\":[{lon},{lat}]}},\"properties\":{{"));
    out.push_str(&format!("\"mmsi\":{},\"shipname\":\"{}\",\"callsign\":\"{}\",\"shiptype\":{},\"shipclass\":{},\"type_class\":\"{}\",\"class_name\":\"{}\",\"country\":\"{}\"",
        v.mmsi, esc(&v.shipname), esc(&v.callsign), v.shiptype, v.shipclass, v.type_class(), v.class_name(), v.country));
    if let Some(s) = v.speed { out.push_str(&format!(",\"speed\":{s:.1}")); }
    if let Some(c) = v.cog { out.push_str(&format!(",\"cog\":{c:.1}")); }
    if let Some(h) = v.heading { out.push_str(&format!(",\"heading\":{h}")); }
    if let Some(s) = v.status { out.push_str(&format!(",\"status\":{s}")); }
    if !v.destination.is_empty() { out.push_str(&format!(",\"destination\":\"{}\"", esc(&v.destination))); }
    if let Some(i) = v.imo { out.push_str(&format!(",\"imo\":{i}")); }
    if let Some(d) = v.draught { out.push_str(&format!(",\"draught\":{d:.1}")); }
    if let Some(a) = v.altitude { out.push_str(&format!(",\"altitude\":{a}")); }
    out.push_str(&format!(",\"count\":{},\"last_signal\":{:.0}", v.count, now - v.last_signal));
    out.push_str("}}");
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
    out.push_str(&format!(",\"shiptype\":{},\"type_class\":\"{}\",\"shipclass\":{},\"class_name\":\"{}\",\"country\":\"{}\"",
        v.shiptype, v.type_class(), v.shipclass, v.class_name(), v.country));
    if let Some(i) = v.imo { out.push_str(&format!(",\"imo\":{i}")); }
    if let Some(d) = v.draught { out.push_str(&format!(",\"draught\":{d:.1}")); }
    if let Some(a) = v.altitude { out.push_str(&format!(",\"altitude\":{a}")); }
    if v.to_bow.is_some() {
        out.push_str(&format!(",\"to_bow\":{},\"to_stern\":{},\"to_port\":{},\"to_starboard\":{}",
            v.to_bow.unwrap_or(0), v.to_stern.unwrap_or(0), v.to_port.unwrap_or(0), v.to_starboard.unwrap_or(0)));
    }
    if v.eta_month.is_some() {
        out.push_str(&format!(",\"eta_month\":{},\"eta_day\":{},\"eta_hour\":{},\"eta_minute\":{}",
            v.eta_month.unwrap_or(0), v.eta_day.unwrap_or(0), v.eta_hour.unwrap_or(0), v.eta_minute.unwrap_or(0)));
        let (m, d, h, mi) = (v.eta_month.unwrap_or(0), v.eta_day.unwrap_or(0), v.eta_hour.unwrap_or(24), v.eta_minute.unwrap_or(60));
        if m > 0 && d > 0 && h < 24 && mi < 60 {
            out.push_str(&format!(",\"eta\":\"{m:02}-{d:02}T{h:02}:{mi:02}Z\""));
        }
    }
    // Computed: length, beam
    if let (Some(bow), Some(stern)) = (v.to_bow, v.to_stern) {
        let len = bow + stern;
        if len > 0 { out.push_str(&format!(",\"length\":{len}")); }
    }
    if let (Some(port), Some(star)) = (v.to_port, v.to_starboard) {
        let beam = port + star;
        if beam > 0 { out.push_str(&format!(",\"beam\":{beam}")); }
    }
    // Status text
    if let Some(s) = v.status {
        let st = match s {
            0 => "Under way using engine", 1 => "At anchor", 2 => "Not under command",
            3 => "Restricted manoeuvrability", 4 => "Constrained by draught",
            5 => "Moored", 6 => "Aground", 7 => "Engaged in fishing",
            8 => "Under way sailing", 14 => "AIS-SART", _ => "",
        };
        if !st.is_empty() { out.push_str(&format!(",\"status_text\":\"{st}\"")); }
    }
    // New decoded fields
    if let Some(t) = v.turn { out.push_str(&format!(",\"turn\":{t}")); }
    if let Some(v) = v.epfd { out.push_str(&format!(",\"epfd\":{v}")); }
    if let Some(v) = v.ais_version { out.push_str(&format!(",\"ais_version\":{v}")); }
    if let Some(v) = v.dte { out.push_str(&format!(",\"dte\":{v}")); }
    if let Some(v) = v.maneuver { out.push_str(&format!(",\"maneuver\":{v}")); }
    if let Some(v) = v.raim { out.push_str(&format!(",\"raim\":{v}")); }
    if let Some(v) = v.virtual_aid { out.push_str(&format!(",\"virtual_aid\":{v}")); }
    if let Some(v) = v.off_position { out.push_str(&format!(",\"off_position\":{v}")); }
    if let Some(v) = v.aid_type { out.push_str(&format!(",\"aid_type\":{v}")); }
    if let Some(v) = v.mothership_mmsi { out.push_str(&format!(",\"mothership_mmsi\":{v}")); }
    out.push_str(&format!(",\"count\":{},\"path_count\":{},\"last_signal\":{:.0}}}", v.count, v.path.len, now - v.last_signal));
}

fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }

fn now_secs() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64()
}

fn mmsi_country(mmsi: u32) -> String {
    let mid = (mmsi / 1000000) % 1000;
    match mid {
        201..=212 => "GR", 213..=214 => "TR", 215 => "MT", 216 => "CY", 218 => "DE",
        219..=220 => "DK", 224..=227 => "ES", 228..=229 => "FR", 230 => "FI",
        231 => "FO", 232..=235 => "GB", 236 => "GI", 237..=241 => "GR", 242 => "MA",
        243 => "HU", 244..=246 => "NL", 247..=249 => "IT", 250 => "IE", 251 => "IS",
        255 => "PT", 256 => "MT", 257..=259 => "NO", 261 => "PL", 263 => "PT",
        265..=266 => "SE", 267 => "CZ", 268 => "UA", 269 => "RU", 270 => "CZ",
        271 => "TR", 272 => "UA", 273 => "RU", 274 => "IS", 275 => "LV",
        276 => "EE", 277 => "LT", 278 => "SI", 279 => "RS",
        301 => "AI", 303..=304 => "US", 305 => "AG", 306 => "CW", 307 => "AW",
        308..=309 | 311 => "BS", 310 => "BM", 312 => "BZ", 314 => "BB",
        316 => "CA", 319 => "KY", 338 | 366..=369 => "US",
        345 => "MX", 351..=354 | 370..=373 => "PA",
        401 => "AF", 403 => "SA", 405 => "BD", 410 => "BT",
        412..=414 => "CN", 416 => "TW", 417 => "LK", 419 => "IN",
        422 => "IR", 425 => "IQ", 428 => "IL", 431..=432 => "JP",
        440..=441 => "KR", 445 => "KP", 447 => "KW", 450 => "LB",
        457 => "MN", 461 => "OM", 463 => "PK", 466 => "QA",
        468 => "SY", 470 => "AE", 477 => "HK",
        501 => "AQ", 503 => "AU", 506 => "MM", 508 => "BN",
        512 => "NZ", 514..=515 => "KH", 525 => "ID", 533 => "MY",
        538 => "MH", 548 => "PH", 553 => "PG", 563..=566 => "SG",
        567 => "TH", 574 => "VN", 576 => "VU",
        601 => "ZA", 603 => "AO", 605 => "DZ", 622 => "EG",
        624 => "ET", 636..=637 => "LR", 647 => "MG", 649 => "ML",
        657 => "NG", 667 => "SL", 672 => "TN", 674 => "TZ",
        _ => "??",
    }.to_string()
}
