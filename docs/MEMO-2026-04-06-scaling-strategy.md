# MEMORANDUM

## Skylink Aircraft Tracker — Scaling Strategy Meeting

**Date:** 2026-04-06
**Attendees:** Project Owner (Decision Maker), Senior Technical Adviser
**Location:** Remote
**Classification:** Internal

---

## 1. Current Production Baseline

| Metric | Value |
|---|---|
| Live URL | skylink.hpradar.com |
| Aircraft tracked | ~10,000 |
| Active feeders | ~2,000 |
| Peak message rate | ~400,000 msg/s |
| Server CPU usage | ~120% (1.2 cores of available) |
| Server RAM usage | ~5 GB |
| Trace storage (/run tmpfs) | ~2 GB |
| Frontend engine | OpenLayers (Canvas 2D, CPU-bound) |
| Backend decoder | readsb (single-threaded) |
| Deployment | Docker, ghcr.io/ngtrthanh/skylink:v1.1.0 |
| Infrastructure | Homelab, Cloudflare tunnel |

**Status:** Production stable. Receiver overlay feature deployed. CI/CD pipeline in place.

---

## 2. Objective

Scale aircraft tracking capacity from 10,000 to 1,000,000 aircraft without exceeding homelab hardware budget.

---

## 3. Identified Bottlenecks

### 3.1 Backend — readsb decoder
- Single-threaded decoding (decode-threads=2 only for >200Mbit/s)
- Aircraft hash table default 65k slots — insufficient for >100k aircraft
- Trace files: 300KB per aircraft in tmpfs — 1M aircraft = 300GB (impossible)
- JSON output: aircraft.json at 1M entries = ~3GB per write cycle

### 3.2 Frontend — OpenLayers
- Creates JavaScript objects per feature (Feature + Style + Geometry)
- 10k aircraft = ~50k JS objects, already causing lag
- 1M aircraft = ~4GB heap → browser out-of-memory
- `Feature.set()` triggers change events — confirmed broken at 2000+ features
- Canvas 2D rendering is CPU-bound, no GPU acceleration

### 3.3 Data Pipeline — JSON polling
- Browser polls full aircraft.json every 1-5 seconds
- At 1M aircraft: ~3GB per poll — network and parse impossible
- No spatial filtering — client receives ALL aircraft regardless of viewport

### 3.4 Storage — trace persistence
- tmpfs-based: limited by RAM
- 1M × 300KB = 300GB traces — exceeds any reasonable tmpfs allocation

---

## 4. Proposed Architecture for 1M Scale

```
                    ┌─────────────────────────────┐
                    │       MapLibre GL JS         │
                    │  (GPU WebGL rendering)       │
                    │  Vector tiles + WS push      │
                    └──────────┬──────────────────┘
                               │
                    ┌──────────┴──────────────────┐
                    │     Tile Server (Go/Rust)    │
                    │  MVT vector tiles            │
                    │  WebSocket delta push        │
                    └──────────┬──────────────────┘
                               │
                    ┌──────────┴──────────────────┐
                    │      Redis / Valkey          │
                    │  Aircraft state store        │
                    │  Geo-indexed by position     │
                    └──────────┬──────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
        readsb-shard1    readsb-shard2    readsb-shard3
        (ICAO 00-55)    (ICAO 55-AA)    (ICAO AA-FF)
              │                │                │
              └────────────────┼────────────────┘
                               │
                     Beast feeders (N thousand)
```

---

## 5. Phased Roadmap

### Phase 0 — Current (COMPLETE)
- 10k aircraft on tar1090 + readsb
- Receiver overlay with geocoding
- Docker image pinned, CI/CD deployed

### Phase 1 — Config tuning to 50k (LOW EFFORT)
**Changes:** readsb config only, no code
- `--ac-hash-bits=18` (256k hash slots)
- `--json-trace-hist-only=1` (no recent traces in tmpfs)
- `--write-json-every=2` (reduce write frequency)
- `--json-trace-interval=120` (less frequent traces)
- `--decode-threads=2`
- `--net-buffer=5`

**Expected outcome:** Handle 50k aircraft on current hardware
**Risk:** Low
**Cost:** Zero

### Phase 2 — MapLibre GL JS frontend (MEDIUM EFFORT)
**Changes:** Replace OpenLayers with MapLibre GL JS
- GPU-rendered aircraft icons (100k+ at 60fps)
- Built-in clustering at low zoom
- Native vector tile support
- Smaller JS bundle (500KB vs 700KB)
- Keep existing data model (planeObject.js, fetchData)
- Rendering layer rewritten entirely

**Expected outcome:** Frontend handles 100k+ aircraft smoothly
**Risk:** Medium — significant frontend rewrite
**Cost:** 2-3 weeks development
**Decision required:** Approve start of proof-of-concept

### Phase 3 — Redis state store + custom tile server (HIGH EFFORT)
**Changes:** New backend architecture
- Redis/Valkey as aircraft state store (replaces JSON files)
- Custom tile server (Go or Rust) generates MVT vector tiles on demand
- WebSocket push for real-time position deltas
- Browser only receives aircraft in current viewport

**Expected outcome:** Handle 500k aircraft
**Risk:** High — new backend components
**Cost:** 4-6 weeks development
**Dependency:** Phase 2 (MapLibre frontend)

### Phase 4 — Sharded readsb (HIGH EFFORT)
**Changes:** Multiple readsb instances
- Shard by ICAO range or geographic region
- Each shard handles ~250k aircraft max
- Router distributes feeders to appropriate shard
- All shards write to shared Redis

**Expected outcome:** Handle 1M+ aircraft
**Risk:** High — distributed system complexity
**Cost:** 2-4 weeks additional
**Dependency:** Phase 3

---

## 6. Cost Estimate (Homelab)

| Component | Current | At 1M |
|---|---|---|
| CPU cores | 1-2 | 8-12 |
| RAM | 8 GB | 32-64 GB |
| Storage | 500 GB HDD | 1 TB NVMe SSD |
| Network | 1 Gbps | 1 Gbps (sufficient) |
| Additional software | None | Redis, custom tile server |
| Cloud cost | $0 (CF free tier) | $0 (CF free tier) |

All runs on homelab. No cloud compute needed.

---

## 7. Key Technical Lessons Learned

From the current development cycle:

1. **Never modify tar1090's `script.js` directly** — breaks cache-bust alignment. All customizations via `config.js`.
2. **OL `Feature.set()` breaks rendering at 2000+ features** — use plain JS properties (`f.idx`).
3. **OL style functions break at scale** — use static styles per layer, separate layers per visual bucket.
4. **readsb port 30005/30006 natively fans out** — no custom splitter needed.
5. **Port 30004 has 2000+ external feeders** — cannot be intercepted by a dumb proxy.
6. **tmpfs for /run needs ~300KB per aircraft** — must be sized accordingly.
7. **Docker `commit` is the simplest way to freeze a customized image** — no Dockerfile needed for tar1090 customizations.

---

## 8. Decisions Required

| # | Decision | Status |
|---|---|---|
| 1 | Approve Phase 1 config tuning | **PENDING** |
| 2 | Approve Phase 2 MapLibre PoC | **PENDING** |
| 3 | Technology choice for tile server (Go vs Rust) | **PENDING** |
| 4 | Redis vs alternative state store | **PENDING** |

---

## 9. Next Actions

| Action | Owner | Deadline |
|---|---|---|
| Apply Phase 1 config changes | Adviser | Upon approval |
| MapLibre GL JS proof-of-concept | Adviser | 2 weeks from approval |
| Benchmark Phase 1 at 50k simulated aircraft | Adviser | 1 week from Phase 1 deploy |

---

**Prepared by:** Senior Technical Adviser
**Approved by:** _____________________ (Project Owner)
**Date:** 2026-04-06
