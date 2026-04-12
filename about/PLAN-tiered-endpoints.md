# PLAN: Tiered Endpoints — Zoom-Adaptive Data Delivery

**Date:** 2026-04-12
**Status:** Draft — ready for implementation

---

## Problem

At low zoom (world view), the frontend loads 7k aircraft + 33k vessels with 37+ fields each.
That's ~15MB JSON per refresh. Most fields are invisible at that zoom level.

## Solution

Three tiers of data density, selected by zoom level or explicit parameter.

---

## Tier Definitions

### Tier 1 — Overview (zoom 0-7, world/continent)

Purpose: dots on map, color by type. Minimal payload.

**Aircraft fields (8):**
```
hex, lat, lon, alt_baro, gs, track, category, type
```

**Vessel fields (7):**
```
mmsi, lat, lon, speed, cog, shipclass, type_class
```

Estimated size: ~120 bytes/aircraft, ~80 bytes/vessel → ~1.2MB for 7k+33k

### Tier 2 — Regional (zoom 8-12, country/region)

Purpose: labels, callsigns, basic info on hover. Medium payload.

**Aircraft fields (18):**
```
hex, flight, lat, lon, alt_baro, alt_geom, gs, track, baro_rate,
squawk, category, type, r, t, seen, seen_pos, messages, rssi
```

**Vessel fields (16):**
```
mmsi, lat, lon, speed, cog, heading, status, status_text,
shipname, shiptype, type_class, shipclass, class_name,
country, length, last_signal
```

Estimated size: ~300 bytes/aircraft, ~200 bytes/vessel → ~3.5MB

### Tier 3 — Detail (zoom 13+, or single target)

Purpose: full data panel, all fields. Current behavior.

**Aircraft:** all 40+ fields
**Vessel:** all 37+ fields

---

## Endpoint Design

### Option A: Query Parameter (recommended)

```
GET /data/aircraft.json?tier=1          # overview
GET /data/aircraft.json?tier=2          # regional
GET /data/aircraft.json                 # full (default, backward compat)
GET /data/aircraft.json?tier=1&box=S,N,W,E  # overview + bbox filter

GET /api/vessels.json?tier=1
GET /api/vessels.json?tier=2
GET /api/vessels.json                   # full

GET /api/vessel?mmsi=123456789          # always tier 3 (single target)
GET /data/traces/{hex}/trace_recent.json  # always full
```

### Option B: Separate Paths

```
GET /data/aircraft.overview.json        # tier 1
GET /data/aircraft.regional.json        # tier 2
GET /data/aircraft.json                 # tier 3
```

**Recommendation:** Option A — single endpoint, query param, backward compatible.

### Bbox Filtering (all tiers)

```
GET /data/aircraft.json?tier=1&box=30,60,-10,40
GET /api/vessels.json?tier=2&box=0,50,90,150
```

Server-side bbox filter reduces payload further. At zoom 10, visible area
might contain 500 aircraft instead of 7000.

### Compressed Variants

All tiers available as zstd:
```
GET /data/aircraft.json.zst?tier=1
```

---

## Implementation Plan

### Phase 1: Server-side tier filtering

1. Define field lists as const arrays per tier
2. In `rebuild_json`, build 3 cached versions (tier 1, 2, 3)
3. Route handler reads `?tier=` param, serves matching cache
4. Default (no param) = tier 3 for backward compat

### Phase 2: Bbox filtering

1. Parse `?box=S,N,W,E` query param
2. Filter aircraft/vessels by lat/lon bounds
3. Bbox + tier combine: `?tier=1&box=...` = smallest possible payload

### Phase 3: Frontend integration

1. Map zoom change → switch tier
2. Zoom 0-7: fetch tier 1 every 2s
3. Zoom 8-12: fetch tier 2 every 1s
4. Zoom 13+: fetch tier 3 every 1s (or WebSocket)
5. Click/hover on target → fetch single detail endpoint

### Phase 4: Binary tier formats

1. binCraft already has a compact mode — map to tier 1
2. Protobuf with field masks per tier
3. GeoJSON tier 1 = minimal properties

---

## Payload Size Estimates

| Scenario | Tier 3 (current) | Tier 2 | Tier 1 |
|---|---|---|---|
| 7k aircraft | ~5 MB | ~2.1 MB | ~0.8 MB |
| 33k vessels | ~10 MB | ~6.6 MB | ~2.6 MB |
| Combined | ~15 MB | ~8.7 MB | ~3.4 MB |
| + zstd | ~3 MB | ~1.8 MB | ~0.7 MB |
| + bbox (zoom 10) | ~0.5 MB | ~0.3 MB | ~0.1 MB |

Tier 1 + zstd + bbox at zoom 10 = **~100KB per refresh**. That's 150x smaller than current.

---

## WebSocket Tiers

```javascript
// Connect with tier preference
ws = new WebSocket('/ws?tier=1')

// Server sends only tier 1 fields in delta updates
// Client can upgrade: send {"tier": 2} message to switch
```

---

## Single Target Detail

Always returns full data regardless of tier:

```
GET /api/aircraft/{hex}         # full aircraft detail
GET /api/vessel?mmsi=MMSI       # full vessel detail (already exists)
GET /data/traces/{hex}/trace_recent.json  # flight path
GET /api/path.geojson?mmsi=MMSI           # vessel track
```

Aircraft single-target endpoint needs to be added (currently only available
via search in the full JSON).

---

## Priority

| Step | Effort | Impact |
|---|---|---|
| Tier 1 + 2 cached JSON | 3 hours | 5-10x smaller payloads |
| Bbox filter on aircraft.json | 2 hours | 10-50x smaller at high zoom |
| Single aircraft endpoint | 1 hour | Detail panel support |
| Frontend zoom→tier mapping | 2 hours | Automatic optimization |
| WebSocket tiers | 3 hours | Live updates at any zoom |
| Binary tier formats | 4 hours | Maximum compression |
