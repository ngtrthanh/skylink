# MEMO: Feature Comparison — skylink-core v3 vs readsb vs readsb-protobuf

**Date:** 2026-04-07  
**From:** Senior Adviser  
**To:** Boss

---

## Overview

| | **readsb** (original) | **readsb-protobuf** (Mictronics fork) | **skylink-core v3** (ours) |
|---|---|---|---|
| Language | C | C + protobuf | Rust |
| Lines of code | ~15K | ~18K | 2,579 |
| Scope | SDR + decoder + aggregator + API | SDR + decoder + aggregator + API + webapp | Aggregator + API only (no SDR) |
| RAM | ~5.4GB | ~5.4GB | **431MB** |
| CPU | ~67% | ~67% | **49%** |

---

## Input Sources

| Feature | readsb | readsb-protobuf | skylink-core v3 |
|---|---|---|---|
| RTL-SDR direct | ✅ | ✅ | ❌ (out of scope) |
| BladeRF | ✅ | ✅ | ❌ |
| PlutoSDR | ✅ | ✅ | ❌ |
| BEAST TCP input | ✅ | ✅ | ✅ |
| Raw input | ✅ | ✅ | ❌ |
| SBS input | ✅ | ✅ | ❌ |
| UAT input | ✅ | ✅ | ❌ |
| ASTERIX input | ❌ | ✅ | ❌ |
| PlaneFinder input | ❌ | ✅ | ❌ |
| GPSD input | ❌ | ✅ | ❌ |
| Multi-receiver | ✅ (net-connector) | ✅ (net-connector) | ❌ (single feed) |

---

## Decoder / Tracker

| Feature | readsb | readsb-protobuf | skylink-core v3 |
|---|---|---|---|
| Mode-S decoding | ✅ Full | ✅ Full | ✅ DF0/4/5/11/17/18/20/21 |
| ADS-B (DF17/18) | ✅ | ✅ | ✅ |
| Mode A/C | ✅ | ✅ | ❌ |
| CPR position decode | ✅ (global + local) | ✅ (global + local) | ✅ (global only) |
| CRC error correction | ✅ | ✅ | ❌ |
| Speed/position filter | ✅ | ✅ | ❌ |
| MLAT forwarding | ✅ | ✅ | ❌ |
| Aircraft DB (hex→type) | ✅ (file-based) | ✅ (file-based) | ✅ (462K, compile-time embedded) |
| Geomagnetic model | ✅ | ✅ | ❌ |

---

## Output Formats — HTTP API

| Format | readsb | readsb-protobuf | skylink-core v3 |
|---|---|---|---|
| aircraft.json | ✅ | ✅ | ✅ |
| aircraft.json.zst | ❌ | ✅ | ✅ |
| binCraft | ✅ | ✅ | ✅ |
| binCraft.zst | ✅ | ✅ | ✅ |
| Protobuf | ❌ | ✅ | ✅ |
| Protobuf.zst | ❌ | ✅ | ✅ |
| Compact binary | ❌ | ❌ | ✅ |
| **GeoJSON** | ❌ | ❌ | ✅ ← new |
| GeoJSON.zst | ❌ | ❌ | ✅ ← new |
| re-api (unified) | ✅ | ✅ | ✅ |
| Bbox filtering | ✅ (globe tiles) | ✅ (globe tiles) | ✅ (query param) |
| **Zero-copy bbox filter** | ❌ | ❌ | ✅ ← new |
| Zstd on all formats | ❌ | Partial | ✅ (every format) |

---

## Output Formats — TCP

| Format | readsb | readsb-protobuf | skylink-core v3 |
|---|---|---|---|
| BEAST output | ✅ | ✅ | ✅ |
| Raw output | ✅ | ✅ | ✅ |
| SBS/BaseStation output | ✅ | ✅ | ✅ |
| SBS Jaero | ❌ | ✅ | ❌ |
| VRS JSON output | ❌ | ✅ | ❌ |
| ASTERIX output | ❌ | ✅ | ❌ |
| JSON position output | ✅ | ✅ | ✅ |
| Beast-reduce output | ✅ | ✅ | ❌ |

---

## WebSocket

| Feature | readsb | readsb-protobuf | skylink-core v3 |
|---|---|---|---|
| WebSocket support | ❌ | ❌ | ✅ ← new |
| **WS binCraft zstd push** | ❌ | ❌ | ✅ ← new |
| Viewport bbox filtering | ❌ | ❌ | ✅ ← new |
| 1s push interval | ❌ | ❌ | ✅ |

---

## Traces

| Feature | readsb | readsb-protobuf | skylink-core v3 |
|---|---|---|---|
| Recent traces | ✅ (file-based) | ✅ (file-based) | ✅ (in-memory ring buffer) |
| Full traces | ✅ (file-based) | ✅ (file-based) | ❌ (recent only) |
| Globe history archive | ✅ (gz per day) | ✅ (gz per day) | ❌ |
| Heatmap generation | ✅ | ✅ | ❌ |
| State persistence | ✅ (disk) | ✅ (disk) | ❌ (in-memory only) |

---

## Globe Index / Tiling

| Feature | readsb | readsb-protobuf | skylink-core v3 |
|---|---|---|---|
| Globe tile files | ✅ (globe_XXXX.binCraft.zst) | ✅ | ❌ |
| Military-only tiles | ✅ (globeMil_XXXX) | ✅ | ❌ |
| Tile-based bbox | ✅ | ✅ | N/A (query-based bbox) |

---

## AI / Modern Features

| Feature | readsb | readsb-protobuf | skylink-core v3 |
|---|---|---|---|
| **MCP (Model Context Protocol)** | ❌ | ❌ | ✅ ← new |
| MCP search (callsign/hex/squawk) | ❌ | ❌ | ✅ |
| MCP area query | ❌ | ❌ | ✅ |
| MCP trace query | ❌ | ❌ | ✅ |
| Prometheus metrics | ❌ | ❌ | ✅ ← new |
| CORS headers | ❌ | ❌ | ✅ |
| Async (tokio) | ❌ | ❌ | ✅ |

---

## Frontend

| Feature | readsb | readsb-protobuf | skylink-core v3 |
|---|---|---|---|
| Bundled frontend | ❌ (uses tar1090) | ✅ (own webapp) | ✅ (skylink-fe, separate repo) |
| Map engine | OpenLayers (tar1090) | OpenLayers | **MapLibre GL** |
| Icon shapes | 81 (tar1090 canvas) | ~20 (webapp) | **81 SDF sprite** |
| Aircraft DB in FE | ✅ (tar1090 db2/) | ✅ (webapp db/) | ✅ (backend-side, sent in binCraft) |
| Real-time update | HTTP polling 1s | HTTP polling 1s | **WebSocket push 1s** |
| Wire format | binCraft.zst polling | JSON polling | **WS binCraft zstd push** |
| Viewport filtering | Client-side | Client-side | **Server-side bbox** |

---

## Performance (measured, same hardware, same BEAST feed)

| Metric | readsb (prod) | skylink-core v3 (staging) |
|---|---|---|
| RAM | 5,437 MB | **431 MB** (12.6x less) |
| CPU | 67% | **49%** (28% less) |
| Aircraft tracked | 6,326 | **7,823** (24% more) |
| Data transfer (per update) | 377 KB (full dump) | **127 KB** (bbox filtered) |
| API latency (bbox binCraft) | 460ms | **350ms** |
| FE page load | 369ms | **241ms** |

---

## What skylink-core v3 Does NOT Have (and whether it matters)

| Missing Feature | Impact | Plan |
|---|---|---|
| SDR direct input | None — we use BEAST from dump1090 | Intentional, out of scope |
| Mode A/C | Low — legacy radar, rare | Not planned |
| CRC error correction | Low — dump1090 handles this | Not planned |
| Local CPR decode | Medium — slightly fewer positions | Week 3 |
| MLAT | Medium — need for multi-receiver | Month 3 |
| Globe tile files | Low — we use query-based bbox | Not planned |
| Full trace archive | Medium — no historical replay | Month 2 |
| State persistence | Medium — cold start loses traces | Month 2 |
| Heatmap | Low — niche feature | Not planned |
| Beast-reduce | Low — for feeding networks | Month 3 |
| VRS/ASTERIX/Jaero | Low — niche protocols | On demand |
| Speed/position filter | Medium — some bad positions | Week 2 |

---

## Summary

skylink-core v3 is a **focused, modern replacement** for the aggregator layer. It doesn't try to replicate readsb's SDR stack or every niche protocol. Instead it:

1. **Does less, but does it better** — 2,579 lines vs 15K+, 12x less RAM
2. **Adds what readsb can't** — WebSocket push, GeoJSON, MCP, Prometheus, zero-copy bbox
3. **Stays compatible** — same binCraft format, same API paths, tar1090 can still connect
4. **Leaves SDR to the experts** — dump1090/readsb handles demod, we consume BEAST

The architecture is: `dump1090 (SDR) → BEAST TCP → skylink-core (aggregator) → skylink-fe (MapLibre)`
