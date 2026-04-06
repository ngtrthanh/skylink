# MEMORANDUM — Session Wrap-up

## Skylink — Development Session Summary

**Date:** 2026-04-06
**Ref:** MEMO-2026-04-06-session-wrapup

---

## What Was Delivered

### Production (deployed, running)
- **Location:** `/opt/workspace/deploy/hpradar.com/skylink/`
- **GitHub:** `ngtrthanh/skylink` main branch, v1.1.0
- **Image:** `ghcr.io/ngtrthanh/skylink@sha256:e000f463...`
- **Status:** Healthy, 10k+ aircraft, 2000+ feeders
- **CI/CD:** GitHub Actions SSH deploy on tag push
- **Features:** Receiver overlay (color-coded, flag-icons, Photon geocoding)

### skylink-core MVP (Rust, proof of concept)
- **Location:** `/opt/workspace/dev/hpradar.com/skylink/skylink-core/`
- **GitHub:** `ngtrthanh/skylink` dev branch
- **Status:** Builds, runs, decodes 5800+ aircraft from live Beast data
- **Docker:** `skylink-core` image, 1.8MB binary
- **Endpoints:** `/data/aircraft.json`, `/data/receiver.json`, `/stats`

### Known Issues with MVP
1. **adsb_deku dependency** — may carry bad design patterns, consider fresh Mode S decoder
2. **JSON serialization at 5k+ aircraft** — serde_json serializing full aircraft list is slow
3. **Missing endpoints** — no binCraft, no globe tiles, no traces, no SBS/VRS output
4. **No receiver tracking** — no feeder quality scoring
5. **No BeastReduce** — no dedup output
6. **CPR decode only** — no speed check, no position reliability scoring

---

## Architecture Decision: Fresh Rewrite

Per project owner's direction, the next version should:

1. **NOT borrow from adsb_deku** — write Mode S decoder from scratch based on ICAO Annex 10 spec
2. **Follow the recommended architecture** from MEMO-2026-04-06-readsb-revamp:
   - Core Domain (aircraft state, position decoder, receiver tracker)
   - Ports (Beast, SBS, ASTERIX protocol interfaces)
   - Adapters (TCP, HTTP, WebSocket, File)
   - Infrastructure (event loop, state store, tile gen, output)
3. **Solve the JSON bottleneck** — use streaming/incremental serialization or binCraft binary format
4. **Match ALL readsb endpoints** — full compatibility with tar1090 frontend

---

## Files on Disk

```
/opt/workspace/
├── deploy/hpradar.com/skylink/     # PRODUCTION (don't touch)
│   ├── docker-compose.yml
│   ├── CHANGELOG.md
│   ├── README.md
│   └── data/                       # volumes
│
└── dev/hpradar.com/skylink/        # DEVELOPMENT
    ├── docs/
    │   ├── MEMO-2026-04-06-scaling-strategy.md
    │   ├── MEMO-2026-04-06-readsb-source.md
    │   ├── MEMO-2026-04-06-readsb-revamp.md
    │   ├── MEMO-2026-04-06-decision-rust-rewrite.md
    │   └── MEMO-2026-04-06-session-wrapup.md
    ├── readsb/                     # reference C source (gitignored)
    └── skylink-core/               # Rust MVP
        ├── src/
        │   ├── main.rs
        │   ├── beast/              # Beast protocol parser + TCP ingest
        │   ├── decode/             # Mode S decoder (adsb_deku, to be replaced)
        │   ├── state/              # Aircraft store + CPR + reaper
        │   ├── api/                # HTTP endpoints
        │   └── output/             # Beast TCP output
        ├── Cargo.toml
        └── Dockerfile
```

---

## Running Containers

| Container | Image | Ports | Purpose |
|---|---|---|---|
| skylink | ghcr.io/ngtrthanh/skylink:v1.1.0 | 31787(web), 30004(beast in), 33005(beast out) | Production |
| skylink-core | skylink-core (local) | 19180(api), 39004(beast in) | Dev/test |

---

## Next Session Priorities

1. Study ICAO Annex 10 / Mode S spec for fresh decoder
2. Design binCraft binary format (or reverse-engineer readsb's)
3. Implement streaming JSON (avoid serializing full aircraft list)
4. Map ALL readsb API endpoints from `api.c` and `json_out.c`
5. Implement globe tile indexing
6. Implement BeastReduce dedup logic

---

**Prepared by:** Senior Technical Adviser
**Date:** 2026-04-06
