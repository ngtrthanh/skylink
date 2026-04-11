# MEMO: v4 Recap + Next Steps — Full-Feature AIS Backend

**Date:** 2026-04-07  
**From:** Senior Adviser  
**To:** Boss

---

## What v4 Has Done (Day 1)

### Module Toggle System
- `skylink.toml` config with `[modules] adsb = true/false, ais = true/false`
- CLI override: `--adsb --ais --no-adsb --no-ais`
- Zero overhead when module is off — no listener, no DashMap, no cache loop, no routes
- Backward compatible: no config = adsb-only (same as v3)

### AIS Aggregator (629 lines of Rust)
- NMEA parser: `!AIVDM/!AIVDO`, multi-sentence, checksum validation
- AIS decoder: types 1-5, 18-19, 21, 24 (covers ~95% of traffic)
- Vessel store: DashMap keyed by MMSI, 10min TTL, auto-reaper
- JSON + GeoJSON builders with 1s pre-built cache
- Bbox filtering on GeoJSON
- TCP ingest from AIS-catcher with auto-reconnect
- WebSocket `/ws/ais` with GeoJSON zstd push

### Live Numbers (after ~1 hour)
- 3,537 vessels tracked, 3,419 with position
- 48K AIS messages processed
- Running alongside 9,910 aircraft on the same binary

### Benchmark vs AIS-catcher
| Metric | AIS-catcher | skylink-core v4 |
|---|---|---|
| JSON latency | 17ms | **0.6ms** (28x faster) |
| JSON size | 2.5MB | **440KB** (5.8x smaller) |
| GeoJSON | ❌ | ✅ 624KB / 0.7ms |
| WebSocket push | ❌ | ✅ zstd binary, 1s, bbox |

---

## What's Missing for Full-Feature AIS

### Priority 1: Parity with AIS-catcher (Week 1)

| Feature | AIS-catcher has | v4 has | Effort |
|---|---|---|---|
| **Vessel paths/tracks** | ✅ Ring buffer per vessel | ❌ | 2h |
| **Path API** | ✅ `/api/path.json?mmsi=X` | ❌ | 1h |
| **All paths GeoJSON** | ✅ `/api/allpath.geojson` | ❌ | 1h |
| **Ship type classification** | ✅ Cargo/tanker/passenger/etc | ❌ | 1h |
| **More AIS message types** | ✅ Types 6-8, 9, 14, 15-17, 27 | ❌ | 3h |
| **Binary messages (DAC/FI)** | ✅ Weather, area notices | ❌ | 4h |
| **Vessel DB (MMSI→name)** | ✅ Loaded from backup | ❌ | 2h |
| **State persistence** | ✅ Save/load to disk | ❌ | 2h |
| **Statistics/histograms** | ✅ msg/sec, per-type counts | ❌ | 1h |
| **Signal quality (ppm, level)** | ✅ Per-vessel | ❌ | 1h |

### Priority 2: Apply ADS-B Wins to AIS (Week 2)

These are features v3 proved work great for aircraft — now apply to vessels:

| ADS-B Feature | AIS Equivalent | Effort |
|---|---|---|
| **binCraft binary format** | **binVessel** — compact binary vessel format, 64-byte stride per vessel | 3h |
| **Zero-copy bbox filter** | Same pattern: pre-encode vessels, filter from cache by lat/lon bytes | 2h |
| **WS binVessel zstd push** | Replace GeoJSON WS with binary — 3-5x smaller per push | 1h |
| **81-shape SDF sprite** | **Vessel sprite** — cargo, tanker, passenger, fishing, sailing, tug, ATON, SAR (~20 shapes) | 4h |
| **Icon resolver (iconMap.js)** | **vesselIconMap.js** — shiptype → icon shape mapping | 2h |
| **Aircraft DB (462K hex→type)** | **Vessel DB** — MMSI→name/flag/type from ITU database | 2h |
| **MCP tool endpoints** | `/mcp/vessel_search`, `/mcp/vessel_area` — AI queries for vessels | 2h |
| **Trace rendering** | Vessel track lines on map, altitude-colored → speed-colored | 1h |

### Priority 3: AIS-Exclusive Features (Week 3)

Things AIS-catcher doesn't have and neither does v3:

| Feature | Description | Effort |
|---|---|---|
| **Unified map** | Aircraft + vessels on same MapLibre canvas, toggle layers | 3h |
| **Unified WebSocket** | Single WS connection pushes both aircraft + vessels | 2h |
| **Collision proximity** | Detect aircraft/vessel proximity (helicopters near ships, SAR ops) | 4h |
| **Port/anchorage detection** | Classify vessels as underway/anchored/moored from position + status | 2h |
| **Voyage tracking** | Track vessel from port A to port B, ETA calculation | 4h |
| **AIS-ADS-B correlation** | Match SAR aircraft with nearby vessels, helicopter-to-platform | 4h |
| **NMEA sentence forwarding** | TCP/UDP output of raw NMEA for downstream consumers | 1h |
| **Multi-source aggregation** | Accept NMEA from multiple AIS-catchers, deduplicate by MMSI | 3h |

---

## Proposed binVessel Format (64 bytes per vessel)

Same philosophy as binCraft — fixed-stride binary, pre-encoded, zero-copy filterable:

```
Offset  Size  Field
0       4     MMSI (u32)
4       4     lat (i32, ×1e6)
8       4     lon (i32, ×1e6)
12      2     SOG (u16, ×10 knots)
14      2     COG (u16, ×10 degrees)
16      2     heading (u16)
18      1     status (u8)
19      1     shiptype (u8)
20      1     shipclass (u8)
21      1     validity bits
22      2     turn rate (i16)
24      4     IMO (u32)
28      20    shipname (20 bytes ASCII)
48      7     callsign (7 bytes ASCII)
55      1     country_mid (u8, MID/10)
56      2     to_bow (u16)
58      2     to_stern (u16)
60      1     draught (u8, ×10)
61      1     reserved
62      2     last_signal (u16, seconds ago)
```

Header (64 bytes): timestamp, vessel count, bbox, version.

Estimated sizes:
- 3,500 vessels × 64 bytes = 224KB raw
- zstd compressed: ~80KB
- Bbox filtered (e.g. Mediterranean): ~30KB

vs current GeoJSON: 624KB raw → ~100KB gzipped

---

## Recommended Execution Order

```
Week 1 (Parity):
  Day 1: Vessel paths + path API + allpath.geojson
  Day 2: Ship type classification + more AIS types (6-8, 9, 27)
  Day 3: Statistics + state persistence + vessel DB

Week 2 (Apply ADS-B wins):
  Day 4: binVessel format + zero-copy bbox filter
  Day 5: WS binVessel zstd + vessel sprite (20 shapes)
  Day 6: vesselIconMap.js + FE vessel layer on MapLibre

Week 3 (Unique features):
  Day 7: Unified map (aircraft + vessels toggle)
  Day 8: Unified WS + MCP vessel endpoints
  Day 9: Multi-source NMEA + NMEA forwarding
```

---

## Architecture (v4 target)

```
RTL-SDR 1090MHz          RTL-SDR 162MHz (×N)
     │                        │
  dump1090               AIS-catcher (×N)
     │                        │
  BEAST TCP              NMEA TCP (×N)
     │                        │
     └────────┬───────────────┘
              │
      skylink-core v4 (Rust, 3,451 lines)
      ├── [adsb] Aircraft store + binCraft + 81 icons
      ├── [ais]  Vessel store + binVessel + 20 icons
      ├── REST API (aircraft + vessels, all formats)
      ├── WS /ws (aircraft binCraft zstd)
      ├── WS /ws/ais (vessel binVessel zstd)
      ├── WS /ws/unified (both, future)
      ├── MCP (aircraft + vessel AI queries)
      └── Prometheus (/stats)
              │
      skylink-fe (MapLibre)
      ├── Aircraft layer (81 shapes, binCraft decoder)
      ├── Vessel layer (20 shapes, binVessel decoder)
      ├── Layer toggle (aircraft/vessels/both)
      └── Unified detail panel
```

---

*v4 Day 1 is done. The AIS aggregator works. Now make it shine with the same optimizations that made v3 beat readsb.*
