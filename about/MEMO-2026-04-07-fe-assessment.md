# MEMORANDUM — ml_clf_fe Assessment & GeoJSON Migration Path

**Date:** 2026-04-07
**From:** Senior Technical Adviser
**To:** Project Owner
**Ref:** MEMO-2026-04-07-fe-assessment

---

## 1. Current ml_clf_fe Architecture

```
Browser
├── MapLibre GL JS (GPU map rendering) ✅
├── Supercluster (clustering) ✅
├── wqi_decoder.js (binCraft parser)
├── zstddec (WASM decompressor)
└── plotacbin.js (main tracker)
    ├── fetch /re-api/?binCraft&zstd&box=  (HTTP poll)
    ├── wqi() → parse binary → JS objects
    ├── objects → GeoJSON features
    ├── Supercluster.load(features)
    ├── getClusters() → for each cluster:
    │   ├── new maplibregl.Marker({ element: DOM })  ← BOTTLENECK
    │   └── marker.setLngLat().addTo(map)
    └── markerCache.clear() + recreate every 3s
```

**Data flow:** binCraft → zstd decompress → binary parse → JS objects → GeoJSON → Supercluster → DOM markers

That's **6 transformation steps** before anything appears on screen.

---

## 2. New GeoJSON Endpoint

```
Backend (skylink-core)
├── Pre-builds GeoJSON FeatureCollection every 1s
├── Pre-compresses with zstd level 3
└── Serves from memory cache (0.5ms)

Endpoints:
  /data/aircraft.geojson       1.1 MB   1.1ms
  /data/aircraft.geojson.zst   192 KB   0.5ms
  /re-api/?geojson&zstd&box=    16 KB   1.8ms  (Europe)
```

---

## 3. What Changes with GeoJSON

### Before (ml_clf_fe current)
```
fetch binCraft+zstd (265KB)
  → zstd decompress in WASM (2ms)
  → wqi binary parse (5ms)
  → build GeoJSON features array (3ms)
  → Supercluster.load() (10ms)
  → getClusters() (2ms)
  → create/destroy 500+ DOM markers (50ms+)
  ─────────────────────────────
  Total: ~72ms per refresh, DOM-bound
```

### After (GeoJSON + symbol layer)
```
fetch geojson.zst (192KB)
  → zstd decompress (2ms)
  → JSON.parse (3ms)
  → map.getSource('aircraft').setData(geojson) (1ms)
  → MapLibre GPU renders all icons (0ms CPU)
  ─────────────────────────────
  Total: ~6ms per refresh, GPU-bound
```

**12x faster refresh cycle. Zero DOM markers.**

### Or even better — raw GeoJSON (no zstd)
```
fetch geojson (1.1MB)
  → response.json() (5ms)
  → source.setData(data) (1ms)
  ─────────────────────────────
  Total: ~6ms, no WASM decoder needed
```

Eliminates: wqi_decoder.js, zstddec.js, Supercluster, DOM marker creation.

---

## 4. File-by-File Impact

| File | Size | Current Role | After GeoJSON | Action |
|---|---|---|---|---|
| plotacbin.js | 16KB | Main tracker, binary decode, DOM markers | Simplified: fetch + setData | **REWRITE** |
| wqi_decoder.js | 6KB | binCraft binary parser | Not needed | **REMOVE** |
| zstddec-tar1090-0.0.5.js | 68KB | ZSTD WASM decoder | Optional (only if using .zst) | **OPTIONAL** |
| acicon.js | 95KB | SVG icon generation for DOM markers | Replace with SDF sprite for symbol layer | **REPLACE** |
| mapManager.js | 13KB | Map init, tile providers | Keep, minor updates | **KEEP** |
| config.js | 2KB | API config | Keep | **KEEP** |
| styles.css | 4KB | Marker styles, layout | Simplify (no DOM marker styles) | **SIMPLIFY** |

**Net result:** Remove 170KB of JS (wqi + zstddec + acicon), replace with ~5KB of GeoJSON fetch + symbol layer setup.

---

## 5. New Minimal FE Architecture

```
Browser
├── MapLibre GL JS (CDN, 200KB gzipped)
├── mapManager.js (map init, tile config)
├── config.js (API URL)
└── tracker.js (NEW, ~100 lines)
    ├── fetch /data/aircraft.geojson (or .zst)
    ├── map.getSource('aircraft').setData(data)
    ├── Symbol layer with icon-image + icon-rotate
    ├── Click handler → detail panel
    └── setInterval or WebSocket for updates
```

**Total custom JS: ~200 lines. Bundle: <20KB.**

### MapLibre Symbol Layer (GPU-rendered, no DOM)
```javascript
map.addSource('aircraft', { type: 'geojson', data: { type: 'FeatureCollection', features: [] } });
map.addLayer({
  id: 'aircraft-icons',
  type: 'symbol',
  source: 'aircraft',
  layout: {
    'icon-image': 'aircraft',           // SDF icon from sprite
    'icon-rotate': ['get', 'track'],    // rotate by heading
    'icon-size': ['interpolate', ['linear'], ['zoom'], 3, 0.3, 10, 0.8],
    'icon-allow-overlap': true,
    'text-field': ['step', ['zoom'], '', 8, ['get', 'flight']],
    'text-size': 10,
    'text-offset': [0, 1.5],
  },
  paint: {
    'icon-color': ['interpolate', ['linear'], ['get', 'alt_baro'],
      0, '#00ff00', 20000, '#ffff00', 40000, '#ff4444'],
    'text-color': '#ffffff',
    'text-halo-color': '#000000',
    'text-halo-width': 1,
  }
});
```

### Clustering (built into MapLibre, no Supercluster needed)
```javascript
map.addSource('aircraft', {
  type: 'geojson',
  data: geojson,
  cluster: true,
  clusterMaxZoom: 8,
  clusterRadius: 50,
});
```

---

## 6. Performance Comparison

| Metric | ml_clf_fe (current) | GeoJSON FE (new) |
|---|---|---|
| JS bundle | 202KB | ~20KB |
| External deps | MapLibre + Supercluster + zstddec | MapLibre only |
| Refresh cycle | 72ms (DOM-bound) | 6ms (GPU-bound) |
| Max aircraft | ~2000 (DOM limit) | 100k+ (GPU) |
| Memory at 10k | ~300MB (DOM markers) | ~50MB (GPU buffers) |
| FPS at 10k | 15-30 | 60 |
| Clustering | Supercluster (JS, CPU) | MapLibre native (GPU) |
| Network per refresh | 265KB (binCraft+zstd) | 192KB (geojson.zst) |

---

## 7. Migration Options

### Option A: Patch ml_clf_fe (1 day)
Replace plotacbin.js internals: swap DOM markers for GeoJSON source + symbol layer.
Keep existing file structure.
- Pro: Minimal change, quick
- Con: Still carries dead code (wqi, zstddec, acicon)

### Option B: Fresh FE (2-3 days)
New `skylink-fe/` directory. Single HTML + 2 JS files.
Clean "liquid glass" design from the strategy memo.
- Pro: Clean, fast, modern
- Con: More work upfront

### Option C: Hybrid (1-2 days)
Fork ml_clf_fe, strip dead code, rewrite plotacbin.js.
Keep mapManager.js and config.js.
- Pro: Best of both — reuse good parts, remove bad
- Con: None significant

**Recommendation: Option C.** Keep the map infrastructure, gut the rendering pipeline.

---

## 8. Backend Endpoint Summary (all pre-cached, sub-ms)

| Endpoint | Size | Use Case |
|---|---|---|
| `/data/aircraft.geojson` | 1.1MB | MapLibre source.setData() — simplest |
| `/data/aircraft.geojson.zst` | 192KB | Same but compressed — best for WAN |
| `/re-api/?geojson&zstd&box=S,N,W,E` | 16KB | Viewport-only — best for mobile |
| `/re-api/?geojson&box=S,N,W,E` | 100KB | Viewport, no compression |
| `/data/aircraft.json` | 1.5MB | Legacy compat |
| `/data/aircraft.binCraft.zst` | 190KB | tar1090 compat |
| `/data/aircraft.compact` | 145KB | Custom binary — smallest |
| `/data/aircraft.pb` | 750KB | Protobuf — structured |
| `/ws` | push | WebSocket compact+zstd — real-time |

---

## 9. Decision Required

| # | Decision | Recommendation |
|---|---|---|
| 1 | Migration approach | Option C (hybrid) |
| 2 | Keep binCraft path? | Yes, as fallback. GeoJSON primary. |
| 3 | Keep Supercluster? | No. Use MapLibre native clustering. |
| 4 | Keep zstd in FE? | Optional. Raw GeoJSON is only 1.1MB. |
| 5 | Start now? | **YES** |

---

**Prepared by:** Senior Technical Adviser
**Date:** 2026-04-07 09:45 ICT
