# MEMORANDUM

## Skylink — readsb Source Code Investigation

**Date:** 2026-04-06
**Ref:** MEMO-2026-04-06-readsb-source
**Attendees:** Project Owner, Senior Technical Adviser
**Purpose:** Understand readsb internals for scaling decisions

---

## 1. Repository

- **Source:** https://github.com/wiedehopf/readsb
- **Local clone:** `/opt/workspace/dev/hpradar.com/skylink/readsb/`
- **Language:** C
- **Total:** 43,263 lines across 66 files
- **Production version:** readsb 3.16.10 (git: 7d341c6)

---

## 2. Key Source Files

| File | Lines | Role | Scaling Relevance |
|---|---|---|---|
| `net_io.c` | 6,453 | Network I/O, Beast protocol, client fan-out | How feeders connect, how data is broadcast |
| `globe_index.c` | 4,092 | Globe tile indexing, trace file writing | Tile generation, trace storage bottleneck |
| `track.c` | 4,041 | Aircraft state tracking, position updates | Core tracking loop, speed check, dedup |
| `readsb.c` | 3,413 | Main loop, init, periodic work | Threading model, main event loop |
| `json_out.c` | 2,361 | JSON output (aircraft.json, receivers.json) | Output generation bottleneck at scale |
| `api.c` | 2,287 | HTTP API (binCraft, globe tiles) | How tar1090 fetches data |
| `aircraft.c` | 1,013 | Aircraft hash table, create/destroy | Hash table sizing, memory per aircraft |
| `receiver.c` | ~800 | Receiver/feeder tracking, bad extent detection | Feeder management at 2000+ |
| `mode_s.c` | 2,243 | Mode-S message decoding | Decode throughput |
| `demod_2400.c` | 787 | SDR demodulation (not used in net-only) | N/A for our setup |

---

## 3. Architecture Overview (from source)

### 3.1 Main Loop (`readsb.c`)
- Single main thread runs `modesNetPeriodicWork()` in a loop
- `--decode-threads=2` adds a second decode thread (only useful >200Mbit/s)
- epoll-based event loop for all network I/O
- Periodic tasks: flush writes, reconnect clients, free closed clients, receiver timeout

### 3.2 Network Model (`net_io.c`)
- **Services:** each port type (beast_in, beast_out, sbs_out, etc.) is a `net_service`
- **Writer pattern:** shared `net_writer` buffer per output service
  - `prepareWrite()` → write to shared buffer
  - `completeWrite()` → mark data ready
  - `flushWrites()` → copy to each client's `sendq`
  - `flushClient()` → `send()` to socket
- **Per-client sendq:** slow clients don't block fast ones
- **Drop-half:** if client can't keep up, drop 50% of packets
- **Heartbeat:** Beast `0x1a 0x31` sent on idle connections
- **epoll:** `EPOLLIN | EPOLLRDHUP | EPOLLERR | EPOLLHUP`, `EPOLLOUT` added only when sendq has pending data

### 3.3 Aircraft Hash Table (`aircraft.c`, `readsb.h`)
- Hash table indexed by ICAO address
- Size: `2^AIRCRAFT_HASH_BITS` (configurable via `--ac-hash-bits`)
- Default: 2^16 = 65,536 slots
- Collision handling: linked list per bucket
- **Scaling limit:** at 100k+ aircraft, collisions increase → O(n) lookup per bucket

### 3.4 Globe Index (`globe_index.c`)
- Divides world into grid tiles (configurable `globeIndexGrid`, default 3°)
- Special tiles for high-density regions (US, EU, Asia)
- Each tile = a JSON or binCraft file written to `/run/readsb/`
- Trace files: one per aircraft, gzipped, written to `/run/readsb/traces/XX/`
- **Scaling limit:** trace files = ~300KB per aircraft in tmpfs

### 3.5 Tracking (`track.c`)
- `trackUpdateFromMessage()` — main entry point for each decoded message
- Position reliability scoring (CPR decoding, speed check)
- Altitude filtering, ground/air state tracking
- `reduce_forward` flag — controls BeastReduce output (dedup)

### 3.6 JSON Output (`json_out.c`)
- `generateAircraftJson()` — writes full aircraft.json
- `generateReceiversJson()` — writes receivers.json
- `sprintAircraftObject()` — formats one aircraft as JSON
- **Scaling limit:** at 1M aircraft, aircraft.json = ~3GB per write

### 3.7 API (`api.c`)
- HTTP API on `--net-api-port`
- Serves binCraft format (compact binary, zstd compressed)
- Globe tile queries: returns aircraft in a specific tile
- **This is how tar1090 gets data** — not via aircraft.json

---

## 4. Scaling-Critical Parameters

| Parameter | Default | Max Tested | Purpose |
|---|---|---|---|
| `--ac-hash-bits` | 16 (65k) | 22 (4M) | Aircraft hash table size |
| `--decode-threads` | 1 | 2 | Decode parallelism |
| `--net-buffer` | 1 (16KB) | 8 (2MB) | Network buffer size: 8KB × 2^n |
| `--json-trace-interval` | 30s | 120s+ | Trace write frequency |
| `--json-trace-hist-only` | 0 | 1,2,3,8 | Skip writing traces to /run |
| `--write-json-every` | 1s | 5s+ | JSON output frequency |
| `--net-ro-size` | 1280 | 8192 | TCP output flush size |
| `--net-beast-reduce-interval` | 0.25s | 15s | BeastReduce dedup interval |

---

## 5. Observations for Scaling

### What readsb does well
- epoll-based networking handles thousands of connections efficiently
- BeastReduce dedup reduces output bandwidth by ~80%
- Globe index tiles enable spatial queries (tar1090 only loads visible tiles)
- binCraft + zstd compression reduces API response size significantly
- Receiver tracking with bad extent detection handles misbehaving feeders

### What limits scaling beyond 100k
- **Single-threaded tracking:** `trackUpdateFromMessage()` processes one message at a time
- **Hash table collisions:** at high fill ratios, linked list traversal slows down
- **Trace storage in tmpfs:** linear with aircraft count, no eviction policy
- **JSON generation:** `aircraft.json` is O(n) for all aircraft, written every second
- **No spatial partitioning for decode:** all messages go through one decode path

### What would need to change for 1M
- Sharded decoding (multiple readsb instances by ICAO range or region)
- External state store (Redis) instead of in-process hash table
- Vector tile generation instead of JSON
- Trace storage on SSD instead of tmpfs
- Or: disable traces entirely, use external flight history service

---

## 6. Next Steps

| # | Action | Status |
|---|---|---|
| 1 | Deep-read `aircraft.c` — hash table implementation | PENDING |
| 2 | Deep-read `globe_index.c` — tile generation logic | PENDING |
| 3 | Deep-read `api.c` — binCraft API format | PENDING |
| 4 | Benchmark readsb with `--ac-hash-bits=20` at current load | PENDING |
| 5 | Test `--json-trace-hist-only=1` impact on tmpfs usage | PENDING |

---

**Prepared by:** Senior Technical Adviser
**For review by:** Project Owner
**Date:** 2026-04-06
