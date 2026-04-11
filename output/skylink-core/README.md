# skylink-core

Rust ADS-B aggregator — drop-in replacement for readsb's network/decode layer.

## Quick Start

### Mode A: Sidecar to skylink (connect to beast output)

```bash
docker compose up -d
```

Default: connects to `skylink:30005` on the `skynet` Docker network.

### Mode B: Direct feeder ingest (replace skylink)

```bash
docker compose -f docker-compose.direct.yml up -d
```

Listens on `:30004` — feeders connect directly, same as readsb.

## Ports

| Port  | Protocol            | Description                |
|-------|---------------------|----------------------------|
| 19180 | HTTP                | API (`/data/aircraft.json`, `/re-api/`, `/stats`) |
| 30004 | Beast TCP input     | Feeders connect here (direct mode) |
| 39004 | Beast TCP input     | Feeders connect here (sidecar mode) |
| 39002 | Raw TCP output      | Hex message per line       |
| 39003 | SBS TCP output      | BaseStation format         |
| 39005 | Beast TCP output    | Binary Beast passthrough   |
| 39006 | BeastReduce output  | Deduplicated Beast         |
| 39047 | JSON TCP output     | Position JSON per line     |

## API Endpoints

```
GET /data/aircraft.json        # All aircraft (JSON, pre-built every 1s)
GET /data/aircraft.binCraft    # All aircraft (binary, 112 bytes/aircraft)
GET /data/receiver.json        # Receiver config (tar1090 compatible)
GET /re-api/?binCraft&box=S,N,W,E          # Bounding box filter
GET /re-api/?binCraft&zstd&box=S,N,W,E     # + zstd compression
GET /stats                     # Aircraft count + message rate
```

## Environment Variables

| Variable        | Default | Description                          |
|-----------------|---------|--------------------------------------|
| `BEAST_CONNECT` | —       | Connect to upstream beast (e.g. `skylink:30005`) |
| `INGEST_PORT`   | 39004   | Beast TCP listen port                |
| `API_PORT`      | 19180   | HTTP API port                        |
| `BASE_PORT`     | 39000   | Base for output ports (+2/+3/+5/+6/+47) |
| `RUST_LOG`      | info    | Log level                            |

## Performance (12k aircraft, 2350 feeders)

| Metric          | readsb     | skylink-core |
|-----------------|------------|--------------|
| JSON response   | 1.5ms      | 1.1ms        |
| binCraft resp   | 2.1ms      | 0.8ms        |
| CPU             | 69%        | 53%          |
| RAM             | 1.6 GB     | 121 MB       |
| Message rate    | —          | 182k msg/s   |

## Build

```bash
cargo build --release
docker build -t skylink-core:v2 .
```
