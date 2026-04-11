# MEMORANDUM — Skylink Next-Gen Frontend Planning

**Date:** 2026-04-07
**From:** Senior Technical Adviser
**To:** Project Owner
**Ref:** MEMO-2026-04-07-frontend-strategy
**Classification:** Internal

---

## 1. Competitive Landscape

### Tier 1 — Commercial
| Tracker | Strengths | Weaknesses |
|---|---|---|
| **Flightradar24** | 3D WebGL (Cesium), aircraft models with liveries, AR mode, 300k+ aircraft, satellite data, airport views, cockpit view | Paywall for premium, heavy JS bundle (~5MB), slow initial load, cluttered UI at low zoom |
| **FlightAware** | Best US coverage, predictive ETAs, historical data, airline integration | Dated UI, no 3D, limited global coverage, ad-heavy free tier |
| **RadarBox** | Clean UI, fleet tracking, airport surface view | Smaller network, less data |

### Tier 2 — Open Source
| Tracker | Strengths | Weaknesses |
|---|---|---|
| **tar1090** | Fast, lightweight, binCraft, good at 10k+ aircraft | OpenLayers (CPU-bound), dated look, no 3D, no clustering |
| **ADSB Exchange** | Unfiltered data, globe view | tar1090 fork, same UI limitations |
| **readsb-protobuf** | Leaflet, protobuf, clean code | Slow with many aircraft, no clustering, abandoned |
| **ml_clf_fe** (ours) | MapLibre GL, Supercluster, binCraft+zstd | Basic, no detail panel, no traces on map |

### What Nobody Does Well
1. **Smooth 60fps at 10k+ aircraft** — everyone lags
2. **Instant detail panel** — FR24 takes 1-2s to load aircraft info
3. **Beautiful traces** — most show ugly polylines
4. **Smart clustering** — either all icons or all clusters, no smooth transition
5. **Dark mode that's actually good** — most are afterthoughts
6. **Mobile-first responsive** — FR24 is good, open source is bad
7. **Real-time without polling** — everyone polls, nobody pushes

---

## 2. Our Unfair Advantages

| Advantage | Detail |
|---|---|
| **Backend speed** | 0.5ms response, pre-cached everything |
| **WebSocket push** | Real-time updates, no polling overhead |
| **Compact format** | 38 bytes/aircraft — 10x smaller than JSON |
| **Full control** | We own BE + FE, can co-design the protocol |
| **No legacy** | Fresh codebase, no backward compat burden |
| **zstd everywhere** | 265KB for 10k aircraft world view |

---

## 3. Design Vision: "Liquid Glass"

### Core Principles
1. **Liquid smooth** — 60fps always, even at 100k aircraft
2. **Information density** — show more data in less space
3. **Progressive disclosure** — zoom reveals detail, not clutter
4. **Dark-first** — aviation is a night activity
5. **Zero chrome** — map is the UI, panels slide in/out

### Visual Language
- Dark map (Carto Dark Matter or custom dark vector tiles)
- Aircraft icons: small, sharp, altitude-colored (blue→green→yellow→red)
- Traces: gradient trails that fade with time, altitude-colored
- Panels: frosted glass (backdrop-filter: blur), slide from edges
- Typography: monospace for data (flight numbers, altitude), sans-serif for labels
- Animations: spring physics for panel transitions, smooth icon rotation

---

## 4. Technical Architecture

```
┌─────────────────────────────────────────────┐
│              Browser (MapLibre GL JS)         │
│                                               │
│  ┌──────────┐  ┌──────────┐  ┌────────────┐ │
│  │ Map Layer │  │ Aircraft │  │  Detail    │ │
│  │ (vector   │  │ Layer    │  │  Panel     │ │
│  │  tiles)   │  │ (WebGL   │  │  (slide)   │ │
│  │           │  │  custom) │  │            │ │
│  └──────────┘  └──────────┘  └────────────┘ │
│        ↑              ↑             ↑        │
│        │         ┌────┴────┐        │        │
│        │         │ Aircraft │        │        │
│        │         │ Store    │        │        │
│        │         │ (Map obj)│        │        │
│        │         └────┬────┘        │        │
│        │              │             │        │
│   MapTiler     WebSocket      REST API       │
│   Free tier    compact+zstd   /re-api/       │
└────────┬──────────────┬─────────────┬────────┘
         │              │             │
    CDN tiles    skylink-core    skylink-core
                   :19180          :19180
```

### Rendering Strategy
- **Zoom 0-4:** Heatmap layer (density, no individual icons)
- **Zoom 4-7:** Clustered dots with count badges
- **Zoom 7-10:** Individual aircraft icons (small triangles)
- **Zoom 10+:** Detailed icons with callsign labels, trace trails
- **All zooms:** Selected aircraft always shown with full detail

### Data Flow
1. **Initial load:** `GET /re-api/?compact&zstd` — full snapshot (~270KB)
2. **Continuous:** WebSocket push compact+zstd every 1s
3. **On click:** `GET /data/traces/{hex}/trace_full.json` — flight path
4. **On search:** `GET /re-api/?find_callsign=XXX&json` — instant results

### Key Technologies
| Component | Technology | Why |
|---|---|---|
| Map engine | MapLibre GL JS v5 | GPU WebGL, free, vector tiles |
| Map tiles | MapTiler (free tier) or PMTiles self-hosted | Dark style, fast |
| Aircraft rendering | Custom WebGL layer or GeoJSON source | 60fps at 10k+ |
| Clustering | Supercluster (Web Worker) | Smooth zoom transitions |
| State management | Vanilla JS Map object | No framework overhead |
| Data transport | WebSocket + compact+zstd | 38 B/aircraft, real-time |
| Decompression | fzstd (JS) | Fast zstd in browser |
| Animations | CSS transforms + requestAnimationFrame | GPU-accelerated |

---

## 5. Feature Roadmap

### Phase 1 — Core Map (1 week)
- [ ] MapLibre GL JS with dark vector tiles
- [ ] WebSocket connection to skylink-core
- [ ] Compact binary decoder in JS
- [ ] Aircraft GeoJSON source with icon layer
- [ ] Altitude-based color scale
- [ ] Smooth icon rotation (heading interpolation)
- [ ] Zoom-dependent rendering (heatmap → clusters → icons)
- [ ] Basic click → detail panel (frosted glass slide-in)

### Phase 2 — Detail & Traces (1 week)
- [ ] Aircraft detail panel: callsign, type, altitude, speed, squawk
- [ ] Flight trace on map (gradient polyline, altitude-colored)
- [ ] Aircraft photo (planespotters.net API or similar)
- [ ] Route line (origin → destination if known)
- [ ] Altitude profile chart (sparkline in panel)
- [ ] Aircraft database integration (type, operator, registration)

### Phase 3 — Polish & Smart Features (1 week)
- [ ] Search bar: flight number, registration, ICAO, airport
- [ ] Filters: altitude range, airline, aircraft type, military
- [ ] Airport overlay: runways, taxiways, gates
- [ ] Weather overlay (METAR/TAF or radar)
- [ ] Day/night terminator on map
- [ ] Responsive mobile layout
- [ ] PWA (installable, offline-capable shell)
- [ ] URL deep linking (share aircraft/view)

### Phase 4 — Wow Factor (1 week)
- [ ] 3D terrain view (MapLibre GL JS terrain)
- [ ] Aircraft trail particles (WebGL custom layer)
- [ ] Playback mode (scrub through history)
- [ ] Multi-aircraft comparison
- [ ] Statistics dashboard (busiest routes, altitude distribution)
- [ ] Notification: emergency squawk, interesting aircraft
- [ ] AR mode (mobile camera + overlay)

---

## 6. Performance Targets

| Metric | Target | FR24 Current |
|---|---|---|
| Initial load | < 1s | 3-5s |
| Time to interactive | < 2s | 5-8s |
| FPS at 10k aircraft | 60 | 15-30 |
| FPS at 100k aircraft | 30+ | N/A (crashes) |
| Data refresh | 1s (WebSocket) | 2-8s (polling) |
| Bundle size | < 500KB gzipped | ~5MB |
| Memory at 10k | < 100MB | ~300MB |
| Click to detail | < 100ms | 1-2s |

---

## 7. What Makes Us Win

| vs FR24 | Our Edge |
|---|---|
| 3-5s load | < 1s (no framework, tiny bundle) |
| Polling every 2-8s | WebSocket push every 1s |
| 5MB bundle | < 500KB |
| Cluttered at low zoom | Heatmap → cluster → icon progression |
| Paywall for features | Everything free, open source |
| No dark mode (free) | Dark-first design |

| vs tar1090 | Our Edge |
|---|---|
| OpenLayers CPU rendering | MapLibre WebGL GPU rendering |
| No clustering | Supercluster with smooth transitions |
| Dated look | Modern "liquid glass" aesthetic |
| No WebSocket | Real-time push |
| No mobile layout | Responsive-first |

---

## 8. Decisions Required

| # | Decision | Options | Recommendation |
|---|---|---|---|
| 1 | Map tile provider | MapTiler free / PMTiles self-hosted / Protomaps | MapTiler free (easiest start, switch to self-hosted later) |
| 2 | Framework | None / Svelte / Solid | None (vanilla JS, smallest bundle) |
| 3 | Aircraft icons | SVG sprites / Canvas / WebGL custom | GeoJSON + symbol layer (MapLibre native, GPU) |
| 4 | Repo structure | Monorepo / Separate FE repo | Monorepo (skylink/fe/) |
| 5 | Name/brand | skylink / hpradar / new name | Decide before public launch |

---

## 9. Next Actions

| Action | Owner | Deadline |
|---|---|---|
| Approve Phase 1 scope | Project Owner | Today |
| Set up FE project structure | Adviser | Upon approval |
| MapTiler API key (free tier) | Project Owner | Today |
| Aircraft icon design (SVG) | Adviser | Day 1 |
| WebSocket compact decoder (JS) | Adviser | Day 1-2 |
| Core map + aircraft layer | Adviser | Day 2-3 |
| Detail panel prototype | Adviser | Day 4-5 |

---

**Prepared by:** Senior Technical Adviser
**Date:** 2026-04-07 07:30 ICT
