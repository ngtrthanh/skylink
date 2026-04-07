# MEMORANDUM — Skylink AI-Powered Aviation Platform

**Date:** 2026-04-07
**From:** Senior Technical Adviser
**To:** Project Owner
**Ref:** MEMO-2026-04-07-ai-platform-strategy
**Classification:** Internal

---

## 1. Vision Shift

Not just a tracker. An **intelligent aviation platform** with edge AI at every layer.

```
Traditional tracker:  feeders → decoder → JSON → map → eyes
Skylink AI platform:  feeders → AI decoder → smart protocol → AI map → AI assistant
```

Every layer gets intelligence:
- **BE:** AI-powered deduplication, anomaly detection, bandwidth optimization
- **Protocol:** Adaptive LOD, delta encoding, predictive compression
- **FE:** Edge LLM for natural language queries, smart rendering, predictive UI

---

## 2. Edge AI with Gemma 4

Gemma 4 E2B (2B params) runs in-browser via WebGPU. No server needed.

### What the Edge LLM Enables

**Natural Language Queries (in browser, offline)**
```
User: "Show me all Emirates flights above FL400 heading to Dubai"
LLM → parses to: { airline: "UAE", alt_min: 40000, dest: "OMDB" }
→ filters aircraft locally, highlights on map

User: "What's that military aircraft circling over the Black Sea?"
LLM → identifies: mil filter + geo circle + pattern detection
→ zooms to area, highlights aircraft, shows orbit pattern

User: "Alert me when any aircraft declares emergency"
LLM → sets up: squawk watch [7500, 7600, 7700]
→ browser notification, no server round-trip
```

**Smart Descriptions (auto-generated)**
```
"Turkish Airlines TK1234, an Airbus A321neo, departed Istanbul 
2 hours ago, currently at FL380 over Romania, descending for 
approach to London Heathrow. Estimated arrival in 1h 42m."
```

**Anomaly Narration**
```
"⚠️ This aircraft (N12345) has been circling at 3000ft for 
25 minutes near a hospital. Likely a medical helicopter or 
law enforcement surveillance pattern."
```

### Implementation
| Component | Technology | Size | Latency |
|---|---|---|---|
| Model | Gemma 4 E2B (quantized INT4) | ~1.2GB download, cached | First load: 5s, then instant |
| Runtime | WebLLM / Transformers.js + WebGPU | — | 50-200ms per query |
| Fallback | Regex pattern matching (no GPU) | 0 | < 1ms |
| Cache | IndexedDB model cache | Persistent | Instant after first load |

---

## 3. Smart Backend — AI at the Aggregator

### 3.1 Intelligent Deduplication
Current: 2000+ feeders send overlapping data. We decode everything.
Smart: ML model scores each feeder's reliability per aircraft, picks best source.

```rust
// Per-aircraft, per-feeder quality score
struct FeederScore {
    latency_ms: f32,      // how fresh
    position_jitter: f32,  // noise level
    coverage_overlap: f32, // redundancy
    reliability: f32,      // historical accuracy
}
// Only decode from top-K feeders per aircraft → 60% less CPU
```

### 3.2 Adaptive Bandwidth
Current: Push full snapshot every 1s to every client.
Smart: Per-client viewport awareness + delta encoding.

```
Client A (zoomed into London): 
  → only aircraft in bbox, full detail, 1s updates
  → ~50 aircraft × 38 bytes = 1.9KB/s

Client B (world view, zoomed out):
  → all aircraft, minimal fields (lat/lon/alt only), 5s updates
  → ~10k aircraft × 12 bytes / 5s = 24KB/s

Client C (tracking specific flight):
  → 1 aircraft, all fields + trace, 0.5s updates
  → ~200 bytes/s
```

### 3.3 Smart Trace Compression
Current: Store every position every 4s (1000 points = 1hr).
Smart: Douglas-Peucker adaptive simplification + curvature-aware sampling.

```
Straight flight: 1 point per 60s (just endpoints)
Turning: 5 points per turn (capture the arc)
Approach: 1 point per 5s (high detail for landing)
Result: 90% fewer points, same visual quality
```

### 3.4 Predictive Position
Between updates, predict aircraft position using:
- Last known velocity + heading
- Great circle interpolation
- Turn rate extrapolation

```
Client sees smooth movement even with 1s update interval.
No "jumping" between positions.
```

---

## 4. Smart Frontend — Adaptive Rendering

### 4.1 Zoom-Adaptive Level of Detail (LOD)

| Zoom | Rendering | Data | Update Rate |
|---|---|---|---|
| 0-3 | Heatmap (density grid) | Aggregated counts per tile | 10s |
| 3-5 | Cluster bubbles | Cluster centroids + counts | 5s |
| 5-8 | Small dots, color = altitude | lat/lon/alt only (12 B/ac) | 3s |
| 8-11 | Triangle icons + callsign | Full fields (38 B/ac) | 1s |
| 11-14 | Detailed icons + trace trail | Full + trace history | 1s |
| 14+ | Airport surface view | Full + ground vehicles | 0.5s |

### 4.2 Smart Trace Rendering
- **Gradient trail:** Color shifts from blue (old) → white (current)
- **Altitude ribbon:** 3D-like ribbon showing altitude changes
- **Speed encoding:** Trail width = ground speed
- **Curvature smoothing:** Bézier interpolation between points
- **LOD:** Simplify trace at low zoom (Douglas-Peucker in shader)

### 4.3 Predictive Rendering
```javascript
// Between WebSocket updates, interpolate position
function interpolatePosition(aircraft, dt) {
  const gs_mps = aircraft.gs * 0.514444; // knots to m/s
  const hdg_rad = aircraft.track * Math.PI / 180;
  const dlat = gs_mps * Math.cos(hdg_rad) * dt / 111320;
  const dlon = gs_mps * Math.sin(hdg_rad) * dt / (111320 * Math.cos(aircraft.lat * Math.PI / 180));
  return { lat: aircraft.lat + dlat, lon: aircraft.lon + dlon };
}
// Smooth 60fps movement even with 1s data updates
```

---

## 5. Smart Protocol — Adaptive Data Delivery

### 5.1 Viewport-Aware WebSocket

```
Client → Server: { "viewport": { "bbox": [S,N,W,E], "zoom": 7 } }

Server decides:
  zoom < 5  → send heatmap grid (256 bytes)
  zoom 5-8  → send compact (lat/lon/alt only, 12 B/ac)
  zoom 8+   → send full compact (38 B/ac)
  
Only aircraft in viewport + 20% margin buffer
```

### 5.2 Delta Encoding
```
Frame 0: Full snapshot (all aircraft in viewport)
Frame 1: Only changed fields since frame 0
  - Aircraft moved: send new lat/lon (8 bytes)
  - Aircraft unchanged: skip (0 bytes)
  - Aircraft entered viewport: full record
  - Aircraft left viewport: remove signal (3 bytes)

Typical delta: 30% of aircraft change per second
→ 70% bandwidth reduction after first frame
```

### 5.3 Priority Encoding
```
High priority (every frame):
  - Selected aircraft
  - Emergency squawk
  - Military
  - Aircraft in approach/departure

Medium priority (every 2nd frame):
  - Aircraft in viewport, moving

Low priority (every 5th frame):
  - Aircraft in viewport, level flight, no changes
  - Aircraft in buffer zone
```

---

## 6. Feeder Intelligence

### 6.1 Smart Feeder Deduplication
Each feeder gets a quality score per aircraft:
- Signal strength (RSSI)
- Position consistency (jitter)
- Message rate
- Latency to aggregator

Aggregator picks best N feeders per aircraft, ignores rest.
Result: Same coverage, 60% less decode work.

### 6.2 Feeder Coverage Optimization
Edge LLM on feeder (Gemma 4 E2B on Raspberry Pi 5):
- Detects coverage gaps
- Suggests antenna adjustments
- Identifies interference patterns
- Auto-tunes gain settings

---

## 7. Revised Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Browser                           │
│                                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ MapLibre  │  │ Aircraft │  │  Gemma 4 E2B     │  │
│  │ GL JS     │  │ Renderer │  │  (WebGPU)        │  │
│  │ (WebGL)   │  │ (adaptive│  │  NL queries      │  │
│  │           │  │  LOD)    │  │  Smart describe  │  │
│  └──────────┘  └──────────┘  │  Anomaly detect   │  │
│       ↑              ↑       └──────────────────┘  │
│       │         ┌────┴────┐         ↑              │
│       │         │ Aircraft │         │              │
│       │         │ Store +  │─────────┘              │
│       │         │ Predictor│                        │
│       │         └────┬────┘                         │
│       │              │                              │
│  Vector tiles   WebSocket (viewport-aware)          │
│  (MapTiler)     delta + priority encoding           │
└───────┬──────────────┬──────────────────────────────┘
        │              │
   CDN tiles    skylink-core (Rust)
                ┌──────┴──────┐
                │ Smart       │
                │ Aggregator  │
                │ - Feeder    │
                │   scoring   │
                │ - Trace     │
                │   compress  │
                │ - Adaptive  │
                │   protocol  │
                │ - Delta     │
                │   engine    │
                └──────┬──────┘
                       │
              2000+ feeders
              (some with edge AI)
```

---

## 8. Implementation Priority

### Sprint 1 — Foundation (1 week)
- [ ] MapLibre GL JS dark map + WebSocket
- [ ] Compact binary decoder in JS
- [ ] Adaptive LOD (heatmap → clusters → icons)
- [ ] Predictive position interpolation (smooth 60fps)
- [ ] Basic detail panel (frosted glass)

### Sprint 2 — Smart Protocol (1 week)
- [ ] Viewport-aware WebSocket (server-side bbox filter)
- [ ] Delta encoding (only send changes)
- [ ] Priority encoding (emergency/selected first)
- [ ] Smart trace compression (Douglas-Peucker on BE)
- [ ] Trace rendering with gradient + altitude color

### Sprint 3 — Edge AI (1 week)
- [ ] Gemma 4 E2B integration (WebLLM)
- [ ] Natural language query → filter pipeline
- [ ] Auto aircraft description generation
- [ ] Anomaly detection (circling, emergency, unusual altitude)
- [ ] Smart search with fuzzy matching

### Sprint 4 — Polish & Wow (1 week)
- [ ] Aircraft photos (planespotters.net)
- [ ] Airport surface view
- [ ] Weather overlay
- [ ] 3D terrain (MapLibre terrain)
- [ ] Playback mode
- [ ] PWA + mobile responsive
- [ ] Performance optimization pass

---

## 9. What Makes This Unprecedented

| Feature | FR24 | tar1090 | ADSB Exchange | **Skylink AI** |
|---|---|---|---|---|
| Edge LLM | ❌ | ❌ | ❌ | ✅ Gemma 4 in browser |
| Natural language query | ❌ | ❌ | ❌ | ✅ "Show Emirates above FL400" |
| Predictive rendering | ❌ | ❌ | ❌ | ✅ Smooth 60fps interpolation |
| Delta encoding | ❌ | ❌ | ❌ | ✅ 70% bandwidth reduction |
| Viewport-aware push | ❌ | ❌ | ❌ | ✅ Only visible aircraft |
| Adaptive LOD | Partial | ❌ | ❌ | ✅ Heatmap→cluster→icon |
| Smart traces | ❌ | ❌ | ❌ | ✅ Douglas-Peucker + curvature |
| Anomaly narration | ❌ | ❌ | ❌ | ✅ AI explains patterns |
| Offline capable | ❌ | ❌ | ❌ | ✅ PWA + edge LLM |
| Sub-500KB bundle | ❌ (5MB) | ❌ (700KB) | ❌ | ✅ ~300KB + lazy LLM |
| Open source | ❌ | ✅ | ✅ | ✅ |

---

## 10. Decisions Required

| # | Decision | Recommendation |
|---|---|---|
| 1 | Edge LLM: Gemma 4 E2B vs Phi-3 mini | Gemma 4 E2B (Apache 2.0, best WebGPU support) |
| 2 | LLM runtime: WebLLM vs Transformers.js | WebLLM (purpose-built for browser, better perf) |
| 3 | LLM loading: eager vs lazy | Lazy (load on first NL query, don't block map) |
| 4 | Delta encoding: binary diff vs field-level | Field-level (simpler, good enough) |
| 5 | Start Sprint 1 now? | **YES** |

---

**Prepared by:** Senior Technical Adviser
**Date:** 2026-04-07 07:35 ICT
