# Changelog

## v1.0.0 (2026-04-06)

Production release.

### Image
- `ghcr.io/sdr-enthusiasts/docker-tar1090@sha256:28717667cd7400b59507572dbb1388d0d79f3474972f682fcb98e9094a3a1331`
- tar1090: 3.14.1740
- readsb: 3.16.10 (wiedehopf git: 7d341c6)

### Features
- Receiver overlay: 2000+ feeder dots on map (LayerSwitcher toggle)
  - Color-coded by data rate (green/orange/red/grey)
  - Hover popup with flag-icons, country (bbox + Photon API), UUID, rate, coverage
  - localStorage geo cache (Photon API only for uncached UUIDs)
- `--write-receiver-id-json` enabled for `/data/receivers.json` API
- nginx gzip tuning (comp_level 4, vary, min_length 256)
- myExtent lat/lon clamping via config.js monkey-patch
- tmpfs /run sized to 3G for globe traces (~300KB/aircraft)

### Infrastructure
- Docker image pinned to sha256 digest (no `:latest` drift)
- GZIP_LVL=1 for faster Beast data response
- --net-buffer 4, --net-connector-delay 5 for high message rate
- ulimit nofile 65536 for 2000+ feeder connections

### Known Limitations
- OL Feature.set() breaks rendering with 2000+ features (use plain properties)
- Never modify script.js directly (breaks cache-bust alignment)
- All FE customizations must go through config.js
