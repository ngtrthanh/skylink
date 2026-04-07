# MEMO: v4 Plan — AIS Aggregator Branch

**Date:** 2026-04-07  
**From:** Senior Adviser  
**To:** Boss

---

## Objective

Add AIS (vessel tracking) to the skylink platform alongside ADS-B (aircraft tracking). Branch `v4` extends skylink-core with a Rust AIS aggregator that replaces AIS-catcher's C++ aggregator layer, same pattern as v3 replaced readsb.

Architecture: `AIS-catcher (SDR/demod) → NMEA TCP → skylink-core v4 (aggregator) → skylink-fe (MapLibre)`

---

## What AIS-catcher Has (146K lines C++)

| Module | Path | What it does | Rewrite? |
|---|---|---|---|
| DSP/Demod | `Source/DSP/` | SDR signal processing, FM demod | ❌ Keep |
| AIS Decoder | `Source/Marine/AIS.cpp` | NRZI → bit stream → AIS messages | ❌ Keep |
| NMEA Parser | `Source/Marine/NMEA.cpp` | `!AIVDM` sentence parsing | ✅ Rewrite |
| Ship Tracker | `Source/Tracking/Ships.cpp, DB.cpp` | Vessel state store, 4096 ships, paths | ✅ Rewrite |
| JSON Builder | `Source/JSON/JSONAIS.cpp` | AIS msg → JSON | ✅ Rewrite |
| HTTP Server | `Source/IO/HTTPServer.cpp` | Custom socket-based HTTP | ✅ Replace (axum) |
| WebViewer | `Source/Application/WebViewer.cpp` | Embedded web UI | ✅ Replace (skylink-fe) |
| PostgreSQL | `Source/DBMS/PostgreSQL.cpp` | Vessel history DB | ⏳ Later |
| SDR Devices | `Source/Device/*.cpp` | RTL-SDR, AirSpy, etc. | ❌ Keep |
| Aviation | `Source/Aviation/` | ADS-B support (Beast/SBS) | ❌ Already in v3 |

---

## AIS-catcher API Endpoints (to replicate)

| Endpoint | Format | Purpose |
|---|---|---|
| `/api/ships.json` | JSON object keyed by MMSI | All vessels |
| `/api/ships_full.json` | JSON with all fields | Full vessel data |
| `/api/ships_array.json` | Compact JSON array | Lightweight polling |
| `/api/path.json?mmsi=X` | JSON path points | Vessel track |
| `/api/allpath.json` | JSON all paths | All vessel tracks |
| `/api/path.geojson?mmsi=X` | GeoJSON LineString | Vessel track |
| `/api/allpath.geojson` | GeoJSON | All tracks |
| `/api/vessel?mmsi=X` | JSON | Single vessel detail |
| `/api/stat.json` | JSON | Station statistics |
| `/api/message?mmsi=X` | JSON | Raw AIS message |
| `/geojson` | GeoJSON | All vessels as GeoJSON |
| SSE `/events` | Server-Sent Events | Real-time updates |

---

## Vessel Data Model (from `Ship` struct)

```rust
struct Vessel {
    mmsi: u32,
    lat: Option<f32>,
    lon: Option<f32>,
    speed: Option<f32>,        // SOG in knots (1/10)
    cog: Option<f32>,          // Course over ground
    heading: Option<u16>,      // True heading
    status: Option<u8>,        // Nav status (0=underway, 1=anchored, 5=moored...)
    shiptype: u8,              // AIS ship type (0-99)
    shipclass: u8,             // Class A/B/basestation/ATON/SAR
    shipname: String,          // 20 chars max
    callsign: String,          // 7 chars max
    destination: String,       // 20 chars max
    imo: Option<u32>,
    draught: Option<f32>,
    to_bow: Option<u16>,       // Dimensions
    to_stern: Option<u16>,
    to_port: Option<u16>,
    to_starboard: Option<u16>,
    eta_month: Option<u8>,
    eta_day: Option<u8>,
    eta_hour: Option<u8>,
    eta_minute: Option<u8>,
    country_code: String,      // Derived from MMSI
    last_signal: f64,          // Timestamp
    count: u32,                // Message count
    level: Option<f32>,        // Signal level
    // v4 additions
    r: Option<String>,         // Registration (from vessel DB)
    t: Option<String>,         // Vessel type name
}
```

---

## Module Toggle System

Idle modules waste CPU (cache rebuild loops, TCP listeners, DashMap overhead). v4 adds a config-driven module system — each module only starts if enabled.

### Config (`skylink.toml`)

```toml
[modules]
adsb = true          # ADS-B aircraft tracking
ais = false          # AIS vessel tracking (off by default)

[adsb]
beast_host = "127.0.0.1:30005"
db = true            # Aircraft type DB (462K entries, ~20MB RAM)

[ais]
nmea_host = "127.0.0.1:10110"
db = true            # Vessel DB (MMSI enrichment)

[api]
port = 19180
formats = ["json", "geojson", "bincraft", "pb"]  # Only register enabled formats
ws = true            # WebSocket push
mcp = true           # MCP AI endpoints
prometheus = true    # /stats endpoint

[outputs]
beast = false        # TCP BEAST output
sbs = false          # SBS/BaseStation output
raw = false          # Raw output
json_pos = false     # JSON position output
```

### What Gets Skipped When Disabled

| Module off | Skipped | RAM saved | CPU saved |
|---|---|---|---|
| `ais = false` | NMEA listener, vessel DashMap, vessel cache loop, AIS decoder, `/api/vessels.*`, `/ws/ais` | ~50MB | ~5% |
| `adsb = false` | BEAST listener, aircraft DashMap, aircraft cache loop, binCraft encoder, `/data/aircraft.*`, `/ws` | ~80MB | ~15% |
| `db = false` (adsb) | 462K aircraft type HashMap | ~20MB | 0% |
| `mcp = false` | MCP routes | 0 | 0% |
| `ws = false` | WebSocket handler, per-client bbox filter loops | ~10MB/client | ~5%/client |
| `beast output = false` | TCP listener + encoder | ~1MB | ~2% |

### Implementation

```rust
// main.rs — conditional module startup
let cfg = Config::load("skylink.toml");

let store = Arc::new(Store::new(&cfg));

if cfg.modules.adsb {
    let s = store.clone();
    tokio::spawn(async move { beast::connect(s, &cfg.adsb.beast_host).await });
    tokio::spawn(async move { cache::rebuild_aircraft_loop(store.clone()).await });
}

if cfg.modules.ais {
    let s = store.clone();
    tokio::spawn(async move { ais::connect(s, &cfg.ais.nmea_host).await });
    tokio::spawn(async move { cache::rebuild_vessel_loop(store.clone()).await });
}

// Router — only register enabled endpoints
let app = api::build_router(&store, &cfg);
```

```rust
// api/mod.rs — conditional route registration
pub fn build_router(store: &Arc<Store>, cfg: &Config) -> Router {
    let mut app = Router::new()
        .route("/stats", get(stats));

    if cfg.modules.adsb {
        app = app
            .route("/data/aircraft.json", get(aircraft_json))
            .route("/re-api/", get(re_api))
            .route("/ws", get(ws::ws_handler));
        // ... all aircraft routes
    }

    if cfg.modules.ais {
        app = app
            .route("/api/vessels.json", get(vessels_json))
            .route("/api/vessels.geojson", get(vessels_geojson))
            .route("/ws/ais", get(ws_ais::ws_handler));
        // ... all vessel routes
    }

    if cfg.api.mcp {
        app = app
            .route("/.well-known/mcp.json", get(mcp::manifest))
            .route("/mcp/search", post(mcp::search));
    }

    app.with_state(store.clone())
}
```

### Store — Conditional Allocation

```rust
pub struct Store {
    // Only allocated if adsb=true
    pub aircraft: Option<DashMap<u32, Aircraft>>,
    pub aircraft_caches: Option<AircraftCaches>,

    // Only allocated if ais=true
    pub vessels: Option<DashMap<u32, Vessel>>,
    pub vessel_caches: Option<VesselCaches>,

    // Always present
    pub messages_total: AtomicU64,
    pub config: Config,
}
```

### CLI Override

```bash
# Enable both
skylink-core --adsb --ais

# AIS only (no aircraft overhead)
skylink-core --ais --no-adsb

# Aircraft only (default, backward compatible with v3)
skylink-core --adsb
```

Config file values are defaults, CLI flags override.

---

## Implementation Plan

### Phase 0: Module Toggle System (Day 1)

- Add `skylink.toml` config parser (use `toml` crate)
- Refactor `main.rs` to conditionally spawn modules
- Refactor `Store` to use `Option<DashMap>` per module
- Refactor `api/mod.rs` to conditionally register routes
- Backward compatible: no config file = adsb=true, ais=false (same as v3)

### Phase 1: NMEA Ingest + Vessel Store (Week 1)

```
skylink-core/src/
├── ais/
│   ├── nmea.rs          # NMEA sentence parser (!AIVDM/!AIVDO)
│   ├── decoder.rs       # AIS message type decoder (1-27)
│   ├── vessel.rs        # Vessel struct + store (DashMap<u32, Vessel>)
│   └── mod.rs           # AIS module root
```

- Parse NMEA `!AIVDM` sentences from TCP (AIS-catcher outputs these)
- Decode AIS message types 1-5, 18-19, 21, 24 (covers 95% of traffic)
- DashMap vessel store keyed by MMSI
- Path/trace ring buffer per vessel

**Input:** TCP connection to AIS-catcher's NMEA output port  
**AIS message types to decode:**

| Type | Name | Fields |
|---|---|---|
| 1,2,3 | Position Report (Class A) | MMSI, lat, lon, SOG, COG, heading, status |
| 5 | Static/Voyage (Class A) | Name, callsign, IMO, type, dimensions, destination, ETA, draught |
| 18 | Position Report (Class B) | MMSI, lat, lon, SOG, COG, heading |
| 19 | Extended Position (Class B) | + name, type, dimensions |
| 21 | Aid to Navigation | MMSI, lat, lon, name, type |
| 24 | Static Data (Class B) | Name, callsign, type, dimensions |

### Phase 2: API + GeoJSON + WebSocket (Week 1-2)

- Add vessel endpoints to existing axum router (alongside aircraft)
- `/api/vessels.json` — all vessels JSON
- `/api/vessels.geojson` — GeoJSON FeatureCollection
- `/ws/ais` — WebSocket binVessel zstd push (same pattern as aircraft)
- Bbox filtering on all endpoints
- Vessel DB (MMSI → name/type/flag) for enrichment

### Phase 3: Frontend — Unified Map (Week 2)

- Add vessel layer to skylink-fe (same MapLibre map)
- Vessel icon sprite (cargo, tanker, passenger, fishing, sailing, tug, etc.)
- Toggle aircraft/vessels/both
- Vessel detail panel (name, MMSI, destination, ETA, dimensions)
- Vessel track rendering

### Phase 4: Combined Platform (Week 3)

- Unified WebSocket: aircraft + vessels in one stream
- MCP endpoints for vessel queries
- Prometheus metrics for AIS
- Vessel history (PostgreSQL, optional)

---

## Architecture (v4)

```
RTL-SDR 1090MHz          RTL-SDR 162MHz
     │                        │
  dump1090               AIS-catcher
  (ADS-B demod)          (AIS demod)
     │                        │
  BEAST TCP              NMEA TCP
     │                        │
     └────────┬───────────────┘
              │
      skylink-core v4 (Rust)
      ├── Aircraft store (DashMap, v3)
      ├── Vessel store (DashMap, v4)
      ├── REST API (aircraft + vessels)
      ├── WS /ws (aircraft binCraft zstd)
      ├── WS /ws/ais (vessel binary zstd)
      └── MCP (aircraft + vessel queries)
              │
      skylink-fe (MapLibre)
      ├── Aircraft layer (81 shapes)
      ├── Vessel layer (ship icons)
      └── Unified map + controls
```

---

## Key Differences from AIS-catcher

| | AIS-catcher | skylink-core v4 |
|---|---|---|
| Language | C++ (146K lines) | Rust (~500 lines added) |
| Scope | SDR + demod + decoder + tracker + web | Aggregator + API only |
| Vessel limit | 4,096 (fixed array) | Unlimited (DashMap) |
| Path limit | 65K points (fixed) | Ring buffer per vessel |
| HTTP | Custom socket server | axum (async, production-grade) |
| Real-time | SSE (Server-Sent Events) | WebSocket binary push |
| Format | JSON only | JSON + GeoJSON + binary + zstd |
| Bbox filter | None | Server-side bbox |
| Aircraft | Basic (via Beast input) | Full v3 aggregator |
| AI/MCP | None | MCP tool endpoints |

---

## Effort Estimate

| Phase | Task | Days |
|---|---|---|
| 0 | Module toggle system (config, conditional spawn, conditional routes) | 1 |
| 1 | NMEA parser + AIS decoder (types 1-5,18,19,21,24) | 2 |
| 1 | Vessel store + path buffer | 1 |
| 2 | API endpoints + GeoJSON + WS | 1 |
| 2 | Vessel DB (MMSI enrichment) | 0.5 |
| 3 | FE vessel layer + icons + panel | 2 |
| 4 | Unified WS + MCP + metrics | 1 |
| | **Total** | **~8.5 days** |

---

## Prerequisites

- AIS-catcher running with NMEA TCP output (`-u 127.0.0.1 10110` or `-S 10110`)
- RTL-SDR tuned to 162.025 MHz (AIS channel)
- v3 battle test passing (don't break aircraft tracking)

---

## Branch Strategy

```
master ─── v3 (aircraft, current) ─── v4 (aircraft + vessels)
```

Create `v4` from `v3` after battle test passes. All v3 aircraft features preserved.

---

*Same playbook as v3: keep AIS-catcher running for SDR/demod, rewrite the aggregator in Rust, serve a modern frontend.*
