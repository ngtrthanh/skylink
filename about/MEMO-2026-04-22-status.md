# MEMO: skylink-core Status — 2026-04-22

**Current version:** v4.5.3 (staging)
**Branch:** v4
**Uptime:** 3+ days continuous

---

## Live Numbers (v4.5.3 staging)

| Metric | skylink-core | readsb |
|---|---|---|
| Aircraft total (store) | **11,320** | 9,630 |
| Aircraft with position | **9,733** | 8,146 |
| Aircraft recent (60s) | 9,682 | — |
| Aircraft recent with pos | 8,476 | 8,146 |
| Vessels tracked | **35,940** | — (no AIS) |
| Vessels with position | **35,387** | — |
| Messages processed | 31.2B | — |
| Uptime | 3.1 days | 15 hours |
| CPU | 231% | 138% |
| Memory | **1.2 GiB** | 5.0 GiB |

skylink-core tracks **1,587 more aircraft with position** than readsb, plus 35k vessels.
Memory is 4x lower. CPU is higher due to serving more clients and building tiered caches.

---

## Version History (v4.x)

| Tag | Date | Key Changes |
|---|---|---|
| v4.0 | Apr 7 | Rust rewrite: Beast ingest, Mode-S, AIS, WS, MCP, binCraft |
| v4.1 | Apr 11 | receivers.json, fasthash64 UUID, multi-source NMEA, BSVDM |
| v4.2 | Apr 12 | **Local CPR** (3-tier: global → aircraft → receiver), allocation reduction |
| v4.3 | Apr 12 | Surface CPR, on_ground, /dashboard, clearer /stats |
| v4.4 | Apr 12 | Full ADS-B JSON field parity (22 fields), AIS field parity (18 fields) |
| v4.5 | Apr 12 | **Tiered endpoints** (?tier=1/2/3), bbox filter, single aircraft endpoint |
| v4.5.1 | Apr 13 | CI/CD GitHub Actions, Docker build workflow |
| v4.5.2 | Apr 13 | Fix stale position filtering in bbox queries |
| v4.5.3 | Apr 13 | Fix seen/seen_pos serialization, dashboard escaping |

---

## What's Built

### ADS-B Engine
- Beast binary ingest (direct + upstream)
- Mode-S decode: DF0/4/5/11/16/17/18/20/21
- CPR: global + aircraft-relative + receiver-relative (airborne + surface)
- 40+ JSON fields matching readsb
- Receiver ID extraction (0xe3 Beast frames, fasthash64 fallback)
- Aircraft DB: 445k aircraft, 2.8k types
- Trace recording: 1000 points per aircraft

### AIS Engine
- Multi-source NMEA TCP (semicolon-separated hosts)
- NMEA parser: AIVDM/AIVDO/BSVDM/BSVDO, metadata prefix stripping
- AIS decode: types 1-9, 11, 14, 18, 19, 21, 24, 27
- 37+ vessel JSON fields
- Vessel state persistence (5-min save, restore on startup)
- Path tracking: 256 points per vessel

### API Layer
- **Tiered endpoints**: ?tier=1 (overview), ?tier=2 (regional), default=full
- **Bbox filtering**: ?box=S,N,W,E on aircraft and vessel endpoints
- 9 pre-built cache formats: JSON, binCraft, protobuf, compact, GeoJSON × plain + zstd
- WebSocket: /ws (aircraft), /ws/ais (vessels), /ws/unified (both)
- MCP: 6 AI tool endpoints
- /dashboard: live HTML status page
- /data/receivers.json: per-feeder stats (readsb format)
- TCP outputs: Beast, SBS, raw, JSON-pos, NMEA forwarding

### Tiered Payload Sizes (current)

| | Tier 1 | Tier 2 | Tier 3 |
|---|---|---|---|
| Aircraft (11k) | 1.6 MB | 2.5 MB | 4.0 MB |
| Vessels (36k) | 3.8 MB | 8.4 MB | 15.7 MB |

---

## Infrastructure

### Repo Structure
```
skylink/ (branch: v4)
├── about/          — 18 docs (memos, API ref, plans)
├── input/          — reference repos (gitignored)
├── output/
│   ├── skylink-core/   — Rust backend
│   ├── skylink-fe/     — MapLibre frontend
│   ├── ml_clf_fe/      — ML classifier frontend
│   └── deploy-template/
└── .github/workflows/  — CI/CD
```

### Workspace
```
/opt/workspace/
├── dev/hpradar.com/skylink/        — development (this repo)
├── staging/hpradar.com/skylink-core/ — staging (v4.5.3 running)
├── deploy/hpradar.com/skylink/     — production (readsb v1.0.1)
└── sandbox/hpradar.com/skylink/    — test runs
```

### Docker Images
- `ghcr.io/ngtrthanh/skylink-core:v4.5.3` — current staging
- `ghcr.io/ngtrthanh/skylink-core:latest`
- `ghcr.io/ngtrthanh/skylink:v1.0.1` — production readsb

---

## What's Next

### In Progress (interrupted)
- Vessel tier caches were being added when session ended
- `json_t1_cache` and `json_t2_cache` added to VesselStore but not initialized/built yet

### Planned
- BDS Comm-B decode (weather: oat, tat, wd, ws; heading: true_heading, roll)
- Lazy cache building (only rebuild formats with active consumers)
- Security hardening (Phase 1: bind internal ports, TLS, rate limiting)
- Frontend zoom→tier auto-switching
- WebSocket tier support

### Known Issues
- CPU higher than readsb (231% vs 138%) — mainly from building 9 cache formats + 3 tiers every second
- BDS decode disabled (mach, true_heading, roll, track_rate from Comm-B not available)
- No dbFlags (military/PIA/LADD) — needs external data source
