# MEMO: skylink-core v4.3 — Engine Status & Frontend Integration Guide

**Date:** 2026-04-12
**Tag:** v4.3
**Status:** Staging — beating readsb on all metrics

---

## What skylink-core Is

A single Rust binary that replaces readsb (C, ADS-B) + AIS-catcher (C++, AIS) + tar1090 API layer. It ingests Beast binary and NMEA TCP streams, decodes Mode-S/ADS-B and AIS messages, maintains an in-memory aircraft + vessel store, and serves data over HTTP, WebSocket, TCP, and MCP.

### Current Performance (v4.3 vs readsb)

| Metric | skylink-core v4.3 | readsb (C) |
|---|---|---|
| Aircraft with position | **5,770** | 4,450 |
| Position decode rate | **84%** | 84% |
| Surface (ground) aircraft | **387** | 386 |
| Vessels tracked | **32,900** | — (no AIS) |
| CPU usage | **93%** | 78% |
| Memory | **906 MiB** | 2.8 GiB |
| Binary size | 1 binary | 50k+ lines C + s6 + nginx |

### What Changed Since v4.0

| Version | Key Changes |
|---|---|
| v4.0 | Initial Rust rewrite: Beast ingest, Mode-S decode, JSON/binCraft/protobuf/GeoJSON, AIS integration, WebSocket, MCP |
| v4.1 | receivers.json, per-frame receiver ID from Beast 0xe3, multi-source NMEA, BSVDM support, 1800s vessel freshness |
| v4.2 | **Local CPR decode** (3-tier: global → aircraft-relative → receiver-relative), hot-path allocation reduction |
| v4.3 | **Surface CPR decode**, on_ground flag, /dashboard endpoint, clearer /stats with recent counts |

---

## Architecture

```
                    ┌─────────────────────────────────────────┐
  Beast feeders ──▶ │  Beast Ingest (:39004)                  │
                    │    ├─ Frame extraction + escape handling │
                    │    ├─ Receiver ID tracking (0xe3 frames) │
                    │    └─ Mode-S decode ──▶ Aircraft Store   │
                    │                          (DashMap)       │
  NMEA sources ──▶  │  AIS Ingest (TCP)                       │
                    │    ├─ NMEA parse (multi-sentence)        │
                    │    ├─ 6-bit decode (types 1-27)          │
                    │    └─ Vessel update ──▶ Vessel Store     │
                    │                          (DashMap)       │
                    │                                          │
                    │  Cache Builder (1s loop)                 │
                    │    └─ JSON, binCraft, protobuf, compact, │
                    │       GeoJSON × plain + zstd             │
                    │                                          │
                    │  HTTP API (:19180)                       │
                    │  WebSocket (/ws, /ws/ais, /ws/unified)   │
                    │  MCP (search, trace, area)               │
                    │  TCP outputs (:39002-39047, :10111)      │
                    └─────────────────────────────────────────┘
```

### CPR Position Decode (the key differentiator)

Three-tier decode, tried in order:
1. **Global CPR** — needs even + odd frame within 10s (standard)
2. **Aircraft-relative** — single frame, uses aircraft's last known position (within 600s)
3. **Receiver-relative** — single frame, uses receiver lat/lon (with range check)

All three support both airborne (360° zones) and surface (90° zones) CPR.

### Data Stores

- **Aircraft Store**: DashMap, 300s reaper TTL, ~7k entries typical
- **Vessel Store**: DashMap, 1800s reaper TTL, ~33k entries typical, persisted to disk every 5 min

---

## Frontend Integration Guide

### Quick Start — What You Need

The backend serves everything on a single HTTP port (default `:19180`). No nginx proxy needed for development. All endpoints return CORS headers.

### Data Formats Available

| Format | Endpoint | Use Case |
|---|---|---|
| JSON | `/data/aircraft.json` | Easy to parse, largest payload |
| binCraft | `/data/aircraft.binCraft` | tar1090 compatible binary, smallest |
| binCraft+zstd | `/data/aircraft.binCraft.zst` | Compressed binary, best for production |
| GeoJSON | `/data/aircraft.geojson` | Direct MapLibre/Mapbox source |
| Protobuf | `/data/aircraft.pb` | Typed schema, good for mobile |
| Compact | `/data/aircraft.compact` | Custom binary, middle ground |

All formats are pre-built and cached — zero compute on request.

### Recommended Frontend Approach

**For MapLibre/Mapbox:**
```javascript
// Option 1: GeoJSON source (simplest)
map.addSource('aircraft', {
  type: 'geojson',
  data: '/data/aircraft.geojson'
});
// Refresh every 1s
setInterval(() => map.getSource('aircraft').setData('/data/aircraft.geojson'), 1000);

// Option 2: WebSocket (lowest latency)
const ws = new WebSocket(`ws://${host}/ws`);
ws.onmessage = (e) => {
  const updates = JSON.parse(e.data);
  // updates = array of aircraft with changed fields only
};
```

**For vessels:**
```javascript
// GeoJSON
map.addSource('vessels', { type: 'geojson', data: '/api/vessels.geojson' });

// WebSocket
const ws = new WebSocket(`ws://${host}/ws/ais`);

// Combined aircraft + vessels
const ws = new WebSocket(`ws://${host}/ws/unified`);
```

### Aircraft JSON Fields

Every aircraft object in `/data/aircraft.json` → `aircraft[]`:

| Field | Type | Description |
|---|---|---|
| `hex` | string | ICAO 24-bit address |
| `flight` | string? | Callsign (trimmed) |
| `alt_baro` | int or "ground" | Barometric altitude (ft) or "ground" |
| `alt_geom` | int? | Geometric altitude (ft) |
| `gs` | float? | Ground speed (knots) |
| `track` | float? | Track angle (degrees) |
| `lat`, `lon` | float? | Position (WGS84) |
| `baro_rate` | int? | Vertical rate (ft/min) |
| `squawk` | string? | Squawk code (octal) |
| `category` | string? | Emitter category (A1-D7) |
| `r` | string? | Registration |
| `t` | string? | Aircraft type (e.g. "B738") |
| `seen` | float | Seconds since last message |
| `seen_pos` | float? | Seconds since last position |
| `rssi` | float? | Signal strength (dBFS) |
| `messages` | int | Message count |

### Vessel JSON Fields

Every vessel in `/api/vessels.json` → `vessels[]`:

| Field | Type | Description |
|---|---|---|
| `mmsi` | int | MMSI identifier |
| `name` | string? | Vessel name |
| `callsign` | string? | Radio callsign |
| `shiptype` | int? | Ship type code |
| `lat`, `lon` | float? | Position |
| `speed` | float? | Speed over ground (knots) |
| `course` | float? | Course over ground (degrees) |
| `heading` | int? | True heading |
| `status` | int? | Navigation status |
| `destination` | string? | Destination port |
| `imo` | int? | IMO number |

### Key Endpoints for Frontend

| What | Endpoint | Notes |
|---|---|---|
| All aircraft | `GET /data/aircraft.json` | Refreshes every 1s server-side |
| Aircraft GeoJSON | `GET /data/aircraft.geojson` | Direct map source |
| Single trace | `GET /data/traces/{hex}/trace_recent.json` | Flight path |
| All vessels | `GET /api/vessels.json` | 30-min window |
| Single vessel | `GET /api/vessel?mmsi=123456789` | Detail view |
| Vessel path | `GET /api/path.geojson?mmsi=123456789` | Track line |
| Live aircraft | `WS /ws` | JSON delta updates |
| Live vessels | `WS /ws/ais` | JSON delta updates |
| Live both | `WS /ws/unified` | Combined stream |
| Server info | `GET /data/receiver.json` | Capabilities, version |
| Health | `GET /stats` | Counts, uptime |
| Dashboard | `GET /dashboard` | HTML status page |
| Sprites | `GET /sprite.json` + `/sprite.png` | Aircraft icons |

### tar1090 Compatibility

skylink-core is a drop-in replacement for readsb's API. Any tar1090 frontend works unchanged:
- `/data/aircraft.json` — same schema
- `/data/aircraft.binCraft` — same binary format
- `/data/receiver.json` — same capabilities object
- `/re-api/` — same query interface
- `/data/traces/{hex}/trace_*.json` — same trace format

---

## Security Safeguard Plan

### Current State (v4.3)

- No authentication on any endpoint
- CORS wide open (`*`)
- Beast ingest accepts any connection
- No rate limiting
- No TLS (handled by reverse proxy)

### Phase 1 — Immediate (low effort)

1. **Reverse proxy hardening** (nginx/caddy in front)
   - TLS termination with auto-renew (Let's Encrypt)
   - Rate limit: 60 req/s per IP on data endpoints, 10 req/s on API
   - Block direct access to ingest port from public network
   - Add `X-Real-IP` forwarding for logging

2. **Network segmentation**
   - Beast ingest port (39004) — bind to internal network only, not 0.0.0.0
   - TCP outputs (39002-39047) — internal only
   - NMEA port (10111) — internal only
   - Only HTTP API port (19180) exposed through reverse proxy

3. **Docker hardening**
   - Drop all capabilities except NET_BIND_SERVICE
   - Read-only root filesystem (tmpfs for /tmp)
   - No new privileges (`--security-opt=no-new-privileges`)
   - Non-root user inside container

### Phase 2 — API Protection (medium effort)

4. **API key authentication**
   - Optional `X-API-Key` header for write/query endpoints
   - MCP endpoints require API key (AI tool access control)
   - Public read endpoints (aircraft.json, geojson) remain open
   - Key rotation via env var, no database needed

5. **WebSocket authentication**
   - Token in query string: `/ws?token=xxx`
   - Validate on upgrade, reject unauthorized
   - Connection limit per IP (e.g. 5 concurrent WS)

6. **Input validation**
   - Beast frame size limit (already 14 bytes max)
   - NMEA line length limit (82 chars per NMEA spec)
   - Query parameter sanitization on re-api, vessel search
   - Reject malformed ICAO hex in trace endpoints

### Phase 3 — Monitoring & Audit (medium effort)

7. **Access logging**
   - Structured JSON logs for all API requests
   - Log feeder connect/disconnect with IP, UUID, duration
   - Alert on unusual patterns (burst connections, scraping)

8. **Health monitoring**
   - `/stats` already provides health data
   - `/data/status.prom` for Prometheus scraping
   - Alert thresholds: aircraft_recent_pos < 1000, uptime resets, memory > 2GiB

9. **Feeder authentication** (future)
   - Beast connection with shared secret (custom 0xe3 handshake)
   - Feeder allowlist by UUID
   - Per-feeder rate tracking and anomaly detection

### Priority Order

| Priority | Item | Effort | Impact |
|---|---|---|---|
| 🔴 P0 | Bind ingest/TCP ports to internal only | 1 hour | Blocks unauthorized feeders |
| 🔴 P0 | Reverse proxy with TLS + rate limit | 2 hours | Protects all public endpoints |
| 🟡 P1 | Docker hardening (non-root, read-only) | 1 hour | Limits blast radius |
| 🟡 P1 | API key for MCP endpoints | 2 hours | Controls AI tool access |
| 🟢 P2 | WebSocket auth + connection limits | 3 hours | Prevents resource exhaustion |
| 🟢 P2 | Prometheus alerting | 2 hours | Operational visibility |
| ⚪ P3 | Feeder authentication | 1 day | Full trust chain |

---

## File Locations

```
about/MEMO-v4.3-engine.md    — this document
about/API.md                 — full endpoint reference
about/DASHBOARD.md           — monitoring guide
output/skylink-core/         — Rust source
output/deploy-template/      — production compose + env
```
