# MEMORANDUM — DECISION RECORD

## Skylink — Rust Aggregator Rewrite: Approved

**Date:** 2026-04-06
**Ref:** MEMO-2026-04-06-decision-rust-rewrite
**Decision by:** Project Owner
**Prepared by:** Senior Technical Adviser

---

## Decisions Made

| # | Decision | Resolution |
|---|---|---|
| 1 | Proceed with revamp? | **YES** |
| 2 | Target language | **Rust** |
| 3 | Scope | Aggregator/decoder layer only. NOT SDR/demod. |
| 4 | Timeline | **1 night** |

## Scope Definition

### IN SCOPE (Rust rewrite)
- Beast TCP ingest (accept feeders on port 30004)
- Mode S frame decoding (via `adsb_deku`)
- Aircraft state management (concurrent hash map)
- Position decoding (CPR)
- BeastReduce-style dedup output
- Beast TCP output (port 30005/30006)
- JSON API (aircraft.json compatible)
- Globe tile API (binCraft or JSON)
- Receiver tracking

### OUT OF SCOPE (stays as-is)
- SDR demodulation (dump1090/readsb on feeder nodes)
- tar1090 frontend (separate project)
- Trace file writing (defer to later)
- Globe history persistence (defer to later)
- Heatmap generation (defer to later)

## Architecture

```
Feeders (readsb/dump1090) → Beast TCP
                              │
                    ┌─────────┴─────────┐
                    │  skylink-core      │
                    │  (Rust + tokio)    │
                    │                    │
                    │  ┌──────────────┐  │
                    │  │ Beast Ingest │  │  ← TCP server, accepts N feeders
                    │  └──────┬───────┘  │
                    │         │          │
                    │  ┌──────┴───────┐  │
                    │  │ Mode S Decode│  │  ← adsb_deku
                    │  └──────┬───────┘  │
                    │         │          │
                    │  ┌──────┴───────┐  │
                    │  │ Aircraft     │  │  ← DashMap (concurrent hash map)
                    │  │ State Store  │  │
                    │  └──────┬───────┘  │
                    │         │          │
                    │  ┌──────┴───────┐  │
                    │  │ Output       │  │  ← Beast out, JSON API, WebSocket
                    │  └──────────────┘  │
                    └────────────────────┘
```

## Key Crates

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime, TCP server/client |
| `adsb_deku` | Mode S / ADS-B frame decoding |
| `dashmap` | Lock-free concurrent hash map for aircraft state |
| `axum` | HTTP API server |
| `serde` / `serde_json` | JSON serialization |
| `tracing` | Structured logging |

## Deliverable

Minimum viable aggregator that:
1. Accepts Beast TCP connections on port 30004
2. Decodes Mode S frames
3. Maintains aircraft state (ICAO, position, altitude, speed, callsign)
4. Serves JSON API compatible with tar1090
5. Outputs Beast on port 30005

---

**Signed:** Project Owner
**Date:** 2026-04-06
**Timeline:** Tonight.
