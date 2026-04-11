# v3 — Full readsb API Parity

## Endpoints to implement

### Static JSON files (written periodically)
| File | Status | Description |
|------|--------|-------------|
| `/data/aircraft.json` | ✅ v2 | All aircraft |
| `/data/aircraft.binCraft` | ✅ v2 | All aircraft binary |
| `/data/aircraft.pb` | ✅ v2 | All aircraft protobuf |
| `/data/aircraft.compact` | ✅ v2 | All aircraft compact |
| `/data/aircraft_recent.json` | ❌ TODO | Aircraft seen in last 60s |
| `/data/receiver.json` | ✅ v2 | Receiver config |
| `/data/status.json` | ❌ TODO | Decoder stats (msg rate, CPR stats, signal) |
| `/data/status.prom` | ❌ TODO | Prometheus metrics |
| `/data/clients.json` | ❌ TODO | Connected feeders list |
| `/data/receivers.json` | ❌ TODO | Receiver UUID list |
| `/data/outline.json` | ❌ TODO | Coverage outline |
| `/data/history_N.json` | ❌ TODO | History snapshots |
| `/data/globe_XXXX.json` | ❌ TODO | Globe tile JSON |
| `/data/globe_XXXX.binCraft` | ❌ TODO | Globe tile binary |
| `/data/globeMil_42777.binCraft` | ❌ TODO | Military filter |

### re-api query interface
| Parameter | Status | Description |
|-----------|--------|-------------|
| `?binCraft` | ✅ v2 | Binary format |
| `?pb` | ✅ v2 | Protobuf format |
| `?compact` | ✅ v2 | Compact format |
| `?json` | ✅ v2 | JSON format |
| `&zstd` | ✅ v2 | Zstd compression |
| `&box=S,N,W,E` | ✅ v2 | Bounding box |
| `&circle=lat,lon,radius` | ❌ TODO | Circle filter |
| `&find_hex=AABBCC` | ❌ TODO | Find by ICAO |
| `&find_callsign=XXX` | ❌ TODO | Find by callsign |
| `&find_reg=N12345` | ❌ TODO | Find by registration |
| `&find_type=B738` | ❌ TODO | Find by type code |
| `&filter_squawk=7700` | ❌ TODO | Filter by squawk |
| `&filter_mil` | ❌ TODO | Military only |
| `&above_alt_baro=N` | ❌ TODO | Altitude filter |
| `&below_alt_baro=N` | ❌ TODO | Altitude filter |
| `&closest=lat,lon` | ❌ TODO | Nearest aircraft |
| `&all` | ✅ v2 | All aircraft |
| `&all_with_pos` | ❌ TODO | All with position |

### Traces
| Endpoint | Status | Description |
|----------|--------|-------------|
| `/data/traces/XX/trace_full_ICAO.json` | ❌ TODO | Full trace |
| `/data/traces/XX/trace_recent_ICAO.json` | ❌ TODO | Recent trace |

### WebSocket
| Endpoint | Status | Description |
|----------|--------|-------------|
| `/ws` | ✅ v2 | Push compact+zstd |

### VRS compatibility
| Endpoint | Status | Description |
|----------|--------|-------------|
| `/VirtualRadar/AircraftList.json` | ❌ TODO | VRS format |
