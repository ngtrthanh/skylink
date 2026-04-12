skylink-core — Dashboard & Monitoring Guide
============================================

Endpoints at a glance
---------------------

  Health & Stats
    GET /stats                  Combined aircraft + vessel store stats
    GET /data/status.json       Aircraft count, messages, uptime, version
    GET /data/status.prom       Prometheus metrics
    GET /api/ais_stats.json     AIS message type breakdown

  Aircraft Data
    GET /data/aircraft.json     All aircraft (JSON, tar1090 compatible)
    GET /data/aircraft.geojson  GeoJSON FeatureCollection
    GET /data/aircraft.binCraft Binary format (tar1090 compatible)
    GET /data/aircraft.pb       Protobuf format
    GET /data/aircraft.compact  Compact binary format
    (all above also available as .zst for zstd-compressed)

  Aircraft Traces
    GET /data/traces/{hex}/trace_full.json
    GET /data/traces/{hex}/trace_recent.json

  Vessel Data
    GET /api/vessels.json       All vessels JSON
    GET /api/vessels.geojson    GeoJSON FeatureCollection
    GET /api/vessel?mmsi=MMSI   Single vessel detail
    GET /api/path.json?mmsi=MMSI        Vessel track
    GET /api/path.geojson?mmsi=MMSI     Vessel track GeoJSON
    GET /api/allpath.geojson    All vessel paths

  Receivers & Clients
    GET /data/receivers.json    Per-feeder stats (UUID, posRate, coverage)
    GET /data/clients.json      Connected feeder list

  WebSocket (live updates)
    WS  /ws                     Aircraft positions
    WS  /ws/ais                 Vessel positions
    WS  /ws/unified             Combined aircraft + vessel

  MCP (AI tool integration)
    GET  /.well-known/mcp.json  Tool manifest
    POST /mcp/search            Search aircraft
    POST /mcp/trace             Flight path history
    POST /mcp/area              Aircraft in bounding box
    GET  /mcp/stats             Stats for AI context
    POST /mcp/vessel_search     Search vessels
    POST /mcp/vessel_area       Vessels in bounding box

  TCP Outputs (BASE_PORT offsets)
    :39002  Raw hex
    :39003  SBS/BaseStation
    :39004  Beast ingest (feeders connect here)
    :39005  Beast output
    :39006  Beast reduce (deduplicated)
    :39047  JSON position lines
    :10111  AIS NMEA forwarding


Understanding /stats
--------------------

  curl http://localhost:19180/stats

  Response:
  {
    "aircraft_total": 6500,       # all aircraft in store (up to 300s old)
    "aircraft_with_pos": 5400,    # aircraft that have decoded a position
    "aircraft_recent": 5200,      # aircraft seen in last 60s
    "aircraft_recent_pos": 4500,  # aircraft with position seen in last 60s
    "aircraft_messages": 30000000,# total messages decoded since start
    "uptime": 3600,               # seconds since start
    "vessel_total": 32000,        # all vessels in store (up to 1800s old)
    "vessel_with_pos": 31000,     # vessels with decoded position
    "vessel_messages": 100000     # total AIS messages decoded
  }

  Notes:
  - aircraft_total includes aircraft up to 300s since last message (reaper TTL)
  - aircraft_recent/aircraft_recent_pos are the "live" counts (last 60s)
  - /data/aircraft.json includes ALL aircraft in the store (same as aircraft_total)
  - vessel_total includes vessels up to 1800s since last signal


Understanding /data/status.json
-------------------------------

  curl http://localhost:19180/data/status.json

  Response:
  {
    "now": 1775983205.1,
    "aircraft_count": 6500,
    "aircraft_count_with_pos": 5400,
    "messages_total": 30000000,
    "uptime": 3600,
    "version": "skylink-core 0.3.0"
  }

  Same as /stats but aircraft-only, readsb-compatible format.


Prometheus Metrics
------------------

  curl http://localhost:19180/data/status.prom

  Output:
    skylink_aircraft_total 6500
    skylink_aircraft_with_pos 5400
    skylink_messages_total 30000000

  Use with Prometheus scrape config:
    - job_name: skylink
      static_configs:
        - targets: ['skylink-core:19180']
      metrics_path: /data/status.prom


AIS Message Stats
-----------------

  curl http://localhost:19180/api/ais_stats.json

  Response:
  {
    "msg_types": {"1":400000,"2":4000,"3":200000,"5":100000,...},
    "class_a": 900000,
    "class_b": 190000,
    "base_station": 30000,
    "aton": 40000,
    "sar": 80
  }


Quick Health Check
------------------

  One-liner to verify the system is healthy:

    curl -sf http://localhost:19180/stats | python3 -c "
    import sys,json; d=json.load(sys.stdin)
    ac=d.get('aircraft_recent_pos',0)
    vs=d.get('vessel_with_pos',0)
    up=d.get('uptime',0)
    print(f'UP {up:.0f}s | aircraft: {ac} with pos | vessels: {vs} with pos')
    ok = ac > 1000 and vs > 10000 and up > 60
    sys.exit(0 if ok else 1)
    "

  Exit code 0 = healthy, 1 = degraded.


Side-by-Side Comparison (vs readsb)
------------------------------------

    echo "skylink-core:" && curl -s http://localhost:19180/stats | python3 -m json.tool
    echo "readsb:" && curl -s http://localhost:31787/data/status.json | python3 -m json.tool

  Key metrics to compare:
  - aircraft_recent_pos vs aircraft_count_with_pos (readsb)
  - Use aircraft_recent_pos for fair comparison (both = last 60s)


Docker Health Check
-------------------

  Add to compose.yaml:

    healthcheck:
      test: ["CMD", "curl", "-sf", "http://localhost:19180/stats"]
      interval: 30s
      timeout: 5s
      retries: 3


Environment Variables
---------------------

  BEAST_CONNECT     Upstream Beast TCP (e.g. skylink:30005)
  INGEST_PORT       Beast ingest port (default: 39004)
  API_PORT          HTTP API port (default: 19180)
  BASE_PORT         TCP output base port (default: 39000)
  NMEA_HOST         AIS NMEA sources, semicolon-separated
  LAT               Receiver latitude (for local CPR decode)
  LON               Receiver longitude (for local CPR decode)
  MAX_RANGE         Max receiver range in NM (default: 300)
  AIS_STATE_PATH    Vessel state file (default: /data/vessels.state)
  RUST_LOG          Log level (default: info)
