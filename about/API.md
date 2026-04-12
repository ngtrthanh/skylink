# skylink-core v4.1 — API & Endpoint Reference

> Rust-native ADS-B + AIS aggregator replacing readsb + AIS-catcher
> Repo: `skylink/skylink-core` branch `v4`

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `BEAST_CONNECT` | *(none)* | Upstream Beast TCP address (e.g. `skylink:30005`). Omit for direct ingest only |
| `INGEST_PORT` | `39004` | Beast TCP ingest port (feeders connect here) |
| `API_PORT` | `19180` | HTTP API port |
| `BASE_PORT` | `39000` | Base for TCP output ports (+2=raw, +3=sbs, +5=beast, +6=beast_reduce, +47=json_pos) |
| `NMEA_HOST` | `127.0.0.1:10110` | AIS NMEA TCP source(s), semicolon-separated (e.g. `host1:5012;host2:5631`) |
| `AIS_STATE_PATH` | `/data/vessels.state` | Vessel state persistence file |
| `RUST_LOG` | `info` | Log level |

---

## TCP Ports (BASE_PORT offsets)

| Port | Offset | Protocol | Description |
|---|---|---|---|
| 39002 | +2 | Raw hex | One hex string per decoded message |
| 39003 | +3 | SBS/BaseStation | SBS-1 format output |
| 39004 | — | Beast binary | Ingest port (feeders connect here) |
| 39005 | +5 | Beast binary | Beast output (re-broadcast) |
| 39006 | +6 | Beast binary | Beast reduce output (deduplicated) |
| 39047 | +47 | JSON lines | Per-aircraft JSON position updates |
| 10111 | — | NMEA text | AIS NMEA sentence forwarding |

---

## HTTP API Endpoints

### ADS-B — Aircraft Data

| Endpoint | Method | Description |
|---|---|---|
| `/data/aircraft.json` | GET | All aircraft, readsb-compatible JSON |
| `/data/aircraft.json.zst` | GET | Zstd-compressed aircraft JSON |
| `/data/aircraft.binCraft` | GET | Binary aircraft format (tar1090 compatible) |
| `/data/aircraft.binCraft.zst` | GET | Zstd-compressed binCraft |
| `/data/aircraft.pb` | GET | Protobuf aircraft format |
| `/data/aircraft.pb.zst` | GET | Zstd-compressed protobuf |
| `/data/aircraft.compact` | GET | Compact binary format |
| `/data/aircraft.geojson` | GET | GeoJSON FeatureCollection of aircraft |
| `/data/aircraft.geojson.zst` | GET | Zstd-compressed GeoJSON |
| `/data/aircraft_recent.json` | GET | Recently updated aircraft |
| `/re-api/` | GET | Unified query endpoint (readsb re-api compatible) |

### ADS-B — Traces

| Endpoint | Method | Description |
|---|---|---|
| `/data/traces/{hex}/trace_full.json` | GET | Full flight trace for ICAO hex |
| `/data/traces/{hex}/trace_recent.json` | GET | Recent trace points for ICAO hex |

### ADS-B — Globe Index (tar1090 compatible)

| Endpoint | Method | Description |
|---|---|---|
| `/data/{*path}` | GET | Globe tile fallback (e.g. `/data/globe_0000.binCraft`) |

### Server Metadata

| Endpoint | Method | Description |
|---|---|---|
| `/data/receiver.json` | GET | Server capabilities (refresh rate, features, version) |
| `/data/receiver.pb` | GET | Protobuf server capabilities |
| `/data/status.json` | GET | Aircraft count, message total, uptime, version |
| `/data/status.prom` | GET | Prometheus metrics format |
| `/stats` | GET | Combined aircraft + vessel statistics |

### Receivers & Clients

| Endpoint | Method | Description |
|---|---|---|
| `/data/receivers.json` | GET | Per-receiver stats array (readsb format): `[uuid, posRate, timeoutRate, latMin, latMax, lonMin, lonMax, badExtent, centerLat, centerLon]` |
| `/data/clients.json` | GET | Connected client list with positions count |

Receiver UUIDs are generated via `fasthash64` of peer address (matching readsb behavior). Feeders sending Beast `0x1a 0xe3` receiver ID frames get their real UUID extracted.

### AIS — Vessel Data

| Endpoint | Method | Description |
|---|---|---|
| `/api/vessels.json` | GET | All vessels JSON (30-min freshness window) |
| `/api/vessels.geojson` | GET | GeoJSON FeatureCollection of vessels |
| `/api/vessel?mmsi={mmsi}` | GET | Single vessel detail by MMSI |
| `/api/path.json?mmsi={mmsi}` | GET | Vessel track path JSON |
| `/api/path.geojson?mmsi={mmsi}` | GET | Vessel track path GeoJSON |
| `/api/allpath.geojson` | GET | All vessel paths GeoJSON |
| `/api/ais_stats.json` | GET | AIS message type counters, class breakdown |

### WebSocket

| Endpoint | Protocol | Description |
|---|---|---|
| `/ws` | WS | Aircraft position updates (JSON) |
| `/ws/ais` | WS | Vessel position updates (JSON) |
| `/ws/unified` | WS | Combined aircraft + vessel updates |

### MCP (Model Context Protocol)

| Endpoint | Method | Description |
|---|---|---|
| `/.well-known/mcp.json` | GET | MCP tool manifest |
| `/mcp/search` | POST | Search aircraft by callsign, ICAO hex, or squawk |
| `/mcp/trace` | POST | Get flight path history |
| `/mcp/area` | POST | List aircraft in bounding box |
| `/mcp/stats` | GET | Aggregator statistics |
| `/mcp/vessel_search` | POST | Search vessels by name, MMSI, type, country |
| `/mcp/vessel_area` | POST | List vessels in bounding box |

### Static Assets

| Endpoint | Method | Description |
|---|---|---|
| `/sprite.json` | GET | Map sprite metadata |
| `/sprite.png` | GET | Map sprite image |

---

## AIS Message Types Decoded

| Type | Name | Fields Extracted |
|---|---|---|
| 1, 2, 3 | Position Report Class A | lat, lon, speed, cog, heading, status, turn |
| 4, 11 | Base Station Report | lat, lon |
| 5 | Static & Voyage (Class A) | name, callsign, IMO, shiptype, dimensions, draught, ETA, destination |
| 6, 7, 8 | Addressed/Binary | MMSI only (presence tracking) |
| 9 | SAR Aircraft Position | lat, lon, speed, cog, altitude |
| 14 | Safety Broadcast | text |
| 18 | Position Report Class B | lat, lon, speed, cog, heading |
| 19 | Extended Position Class B | lat, lon, speed, cog, heading, name, shiptype, dimensions |
| 21 | Aid to Navigation | lat, lon, name, type |
| 24 | Static Data Class B | Part A: name / Part B: shiptype, callsign, dimensions |
| 27 | Long-Range (satellite) | lat, lon, speed, cog, status |

### NMEA Parser Features
- Handles `!AIVDM`, `!AIVDO`, `!BSVDM`, `!BSVDO` sentence types
- Strips metadata prefixes (`\s:...,c:...*xx\`) from community feeds
- Multi-sentence message reassembly
- Checksum verification

---

## Beast Protocol Features

- Standard Beast binary frames: types `0x31` (Mode-AC), `0x32` (Mode-S short), `0x33` (Mode-S long)
- Receiver ID extraction from `0x1a 0xe3` frames (8-byte big-endian u64, escape-aware)
- Per-frame receiver ID tracking for aggregated streams
- Auto-reconnect on upstream disconnect (5s interval)

---

## Vessel Store

- **Reaper TTL**: 1800s (30 min) — vessels removed after no signal
- **JSON freshness**: 1800s — all vessels in store appear in API responses
- **State persistence**: saves to `AIS_STATE_PATH` every 5 min, restores on startup
- **Multi-source**: semicolon-separated `NMEA_HOST` spawns parallel ingest tasks

---

## Docker

```yaml
services:
  skylink-core:
    image: ghcr.io/ngtrthanh/skylink-core:v4.1
    command: ["skylink-core", "--ais"]
    environment:
      - BEAST_CONNECT=skylink:30005        # or omit for direct ingest
      - INGEST_PORT=30004
      - API_PORT=19180
      - BASE_PORT=39000
      - NMEA_HOST=host1:5012;host2:5631
      - AIS_STATE_PATH=/data/vessels.state
    ports:
      - "19180:19180"
      - "30004:30004"
    volumes:
      - data:/data
```

---

## Version History

| Tag | Date | Highlights |
|---|---|---|
| v4.5.3 | 2026-04-12 | Fixed tier 1/2 frontend timeout issue: explicitly serialize `seen` and `seen_pos`, and strict bounds filtering for stale signals |
| v4.5 | 2026-04-12 | Aircraft tiered JSON endpoints (`?tier=1`, `?tier=2`), bounds filtering (`?box=S,N,W,E`) directly injected into APIs, surface CPR decode |
| v4.1 | 2026-04-11 | receivers.json with fasthash64 UUID, per-frame receiver ID from Beast 0xe3, multi-source NMEA, BSVDM support, 1800s vessel freshness |
| v4.0 | 2026-03-28 | v4 rewrite: unified WS, MCP endpoints, AIS integration, vessel sprites, state persistence |
| v2-final | — | Legacy C-based version |
