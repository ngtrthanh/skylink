# MEMORANDUM

## Skylink — readsb Codebase Evaluation & Revamp Plan

**Date:** 2026-04-06
**Ref:** MEMO-2026-04-06-readsb-revamp
**Attendees:** Project Owner (Decision Maker), Senior Technical Adviser
**Purpose:** Evaluate readsb code quality, plan modernization path

---

## 1. Code Quality Assessment

### Verdict: Legacy C monolith, functional but unmaintainable at scale

| Metric | Value | Assessment |
|---|---|---|
| Total lines | 43,263 | Medium codebase |
| God struct (`Modes`) | **631 fields** | Catastrophic — single global state object |
| Longest function | **970 lines** (`trackUpdateFromMessage`) | Unmaintainable |
| Files including `readsb.h` | **27 of 33** .c files | Everything coupled to everything |
| Static functions in `net_io.c` | 111 | File is doing too much |
| Functions >200 lines | 12+ | No separation of concerns |
| Tests | **0** (only `cprtests.c`) | Effectively untested |
| Error handling | `fprintf(stderr)` + continue | No structured error handling |
| Memory management | Manual `malloc`/`free` | Leak-prone, no RAII |
| Threading | Mutex-based, 1-2 threads | Not designed for parallelism |

### Specific Problems

**1. God Struct (`Modes` — 631 fields)**
```c
struct {
    // 631 fields covering:
    // - SDR config
    // - Network state
    // - Aircraft hash table
    // - JSON output config
    // - Globe index state
    // - Stats counters
    // - Receiver table
    // - API state
    // - Debug flags
    // ... everything in one struct
} Modes;
```
Every file reads/writes `Modes.*` directly. No encapsulation. Impossible to test in isolation.

**2. Monster Functions**
| Function | Lines | File |
|---|---|---|
| `trackUpdateFromMessage()` | 970 | track.c |
| `modesSendAsterixOutput()` | 533 | net_io.c |
| `decodeAsterixMessage()` | 485 | net_io.c |
| `traceAddInternal()` | 481 | globe_index.c |
| `mark_legs()` | 462 | globe_index.c |
| `readBeast()` | 374 | net_io.c |
| `init_globe_index()` | 345 | globe_index.c |
| `decodeSbsLine()` | 300 | net_io.c |

**3. Mixed Concerns in Single Files**
- `net_io.c` (6,453 lines): Beast decode + SBS decode + ASTERIX decode + JSON output + GPS handling + client management + heartbeat + ping/pong + UUID parsing
- `globe_index.c` (4,092 lines): tile indexing + trace writing + heatmap + state persistence + leg marking

**4. No Abstraction Boundaries**
- No interfaces/traits — everything calls everything directly
- Protocol decoding mixed with network I/O
- Output formatting mixed with state management
- No dependency injection — all hardcoded to `Modes.*`

---

## 2. What Works Well (Keep)

Despite the mess, the core algorithms are battle-tested:

| Component | Quality | Notes |
|---|---|---|
| Beast protocol decode | Excellent | Handles edge cases, escape sequences, receiver IDs |
| CPR position decoding | Excellent | Robust with speed check, reliability scoring |
| BeastReduce dedup | Excellent | Efficient per-aircraft rate limiting |
| Globe tile indexing | Good | Spatial partitioning works well for tar1090 |
| epoll networking | Good | Handles 2000+ connections efficiently |
| Receiver tracking | Good | Bad extent detection, RTT-based quality scoring |

---

## 3. Revamp Options

### Option A: Refactor in C
- Extract `Modes` into domain-specific structs
- Split monster functions
- Add unit tests with a test framework
- **Pros:** Minimal risk, incremental
- **Cons:** Still C, still manual memory, still no type safety
- **Effort:** 4-6 weeks
- **Verdict:** Polishing a legacy codebase. Not worth it for 1M scale.

### Option B: Rewrite in Rust
- Type safety, memory safety, zero-cost abstractions
- `tokio` for async networking (replaces epoll manually)
- `serde` for serialization (replaces manual JSON)
- Trait-based protocol handlers (Beast, SBS, ASTERIX)
- Fearless concurrency for sharded decoding
- **Pros:** Best long-term choice, safety guarantees, excellent performance
- **Cons:** Steep learning curve, longest rewrite time
- **Effort:** 8-12 weeks for core functionality
- **Verdict:** Best for a ground-up rewrite targeting 1M.

### Option C: Rewrite in Go
- Simple concurrency (goroutines + channels)
- Fast development, easy to read
- Good networking stdlib
- **Pros:** Fastest to develop, easy to hire for
- **Cons:** GC pauses at high throughput, higher memory usage
- **Effort:** 6-8 weeks for core functionality
- **Verdict:** Good for rapid prototyping, may hit GC wall at 1M.

### Option D: Rewrite in Zig
- C-level performance, no hidden allocations
- Comptime for protocol parsing
- Easy C interop (can wrap existing readsb incrementally)
- **Pros:** Performance of C with better ergonomics, can call existing C code
- **Cons:** Small ecosystem, fewer libraries, immature tooling
- **Effort:** 8-10 weeks
- **Verdict:** Interesting for incremental migration from C.

---

## 4. Recommended Architecture (Language-Agnostic)

Regardless of language choice, the new architecture should be:

```
┌─────────────────────────────────────────────────┐
│                   Core Domain                    │
│                                                  │
│  Aircraft State    Position Decoder    Receiver  │
│  (hash map)        (CPR, speed check)  Tracker   │
│                                                  │
├─────────────────────────────────────────────────┤
│                   Ports (Interfaces)             │
│                                                  │
│  BeastPort    SBSPort    ASTERIXPort    APIPort  │
│  (decode)     (decode)   (decode)       (serve)  │
│                                                  │
├─────────────────────────────────────────────────┤
│                   Adapters                       │
│                                                  │
│  TCP Server    TCP Client    HTTP API    File    │
│  (feeders)     (connectors)  (tiles)     (trace) │
│                                                  │
├─────────────────────────────────────────────────┤
│                   Infrastructure                 │
│                                                  │
│  Event Loop    State Store    Tile Gen    Output  │
│  (epoll/tokio) (memory/Redis) (MVT/JSON) (Beast) │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Design Principles
1. **No god struct** — each domain has its own state
2. **Protocol decode separated from I/O** — pure functions that take bytes, return messages
3. **State store is pluggable** — in-memory hash map or Redis, same interface
4. **Output is pluggable** — JSON, binCraft, MVT, WebSocket, same interface
5. **Testable** — every component testable in isolation with mock inputs
6. **Shardable** — aircraft state can be partitioned by ICAO range

---

## 5. Migration Strategy

### Phase 1: Extract & Wrap (2 weeks)
- Extract core algorithms from readsb into clean C libraries:
  - `beast_decode.c` — pure Beast protocol parser
  - `cpr_decode.c` — CPR position decoder (already somewhat isolated)
  - `speed_check.c` — position validation
- Wrap with clean C headers, no `Modes.*` dependency
- These can be called from any language via FFI

### Phase 2: New Shell (4 weeks)
- Write new main application in chosen language (Rust/Go/Zig)
- Use FFI to call extracted C libraries for decode
- New networking layer (async, multi-threaded)
- New state store (concurrent hash map)
- New API layer (HTTP + WebSocket)

### Phase 3: Replace C Core (4 weeks)
- Rewrite Beast decode in target language (eliminate FFI)
- Rewrite CPR decode in target language
- Rewrite speed check in target language
- Now fully native, no C dependency

### Phase 4: Scale Features (2 weeks)
- Sharded state store
- Vector tile generation
- WebSocket push
- Clustering for frontend

---

## 6. Adviser Recommendation

**Go with Rust** if this is a long-term project you want to maintain for years. The safety guarantees and performance characteristics are ideal for a high-throughput data pipeline handling 1M aircraft.

**Go with Go** if you want something working in 6 weeks and can accept the GC overhead. Good enough for 500k, may struggle at 1M.

**Don't refactor the existing C** — the architecture is the problem, not the syntax. Refactoring C into better C still leaves you with manual memory management, no type safety, and no concurrency model.

---

## 7. Decisions Required

| # | Decision | Options | Status |
|---|---|---|---|
| 1 | Proceed with revamp? | Yes / No / Defer | **PENDING** |
| 2 | Target language | Rust / Go / Zig | **PENDING** |
| 3 | Migration strategy | FFI wrap first / Clean rewrite | **PENDING** |
| 4 | Timeline commitment | 8 weeks / 12 weeks / open-ended | **PENDING** |

---

**Prepared by:** Senior Technical Adviser
**For decision by:** Project Owner
**Date:** 2026-04-06
