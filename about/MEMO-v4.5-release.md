# MEMO: v4.5.3 Tiered Endpoints & Map Optimization Release

**Date:** 2026-04-12  
**Status:** Released

---

## Overview

Version 4.5 introduces **Tiered Endpoints** for the `skylink-core` API to drastically diminish the payload size sent to clients requesting the global state, ensuring stability when tracking high volumes (7,000+ aircraft & 30,000+ vessels).

The changes also ensure accurate representation of inactive track timeouts, which had a few issues post-release that were patched in the subsequent `v4.5.2` and `v4.5.3` updates.

## Features Added

### 1. Endpoint Tiers
Data density is now directly accessible using the `?tier=` query logic across JSON aircraft API queries:
*   **Tier 1 (`?tier=1`)**: Drops almost all heavy metadata strings. Optimized for zoomed-out world map plotting (only `hex`, `lat`, `lon`, `alt_baro`, `gs`, `track`, `category`, and signal `type`).
*   **Tier 2 (`?tier=2`)**: Regional. Carries callsigns, basic diagnostics and hover info.
*   **Tier 3 (default)**: Detailed view matching the exact behavior of `< v4.4` returning exhaustive datasets covering ~40 fields per aircraft.

### 2. Live Bounding Box Filtering
Combined with the unified API wrapper `/re-api/`, the global `aircraft.json` HTTP paths now process `?box=S,N,W,E` variables internally, drastically speeding up payload returns by sending datasets natively scoped to viewport requests.

### 3. Surface Position Decode
Local and global ground/surface vehicle decoding functions in CPR have been successfully expanded, processing ADS-B tracks effectively regardless of altitude profile.

---

## Patches (v4.5.1 - v4.5.3)

#### v4.5.1
*   **GH Actions Package Workflow**: Patched `GITHUB_TOKEN` explicit container-package registry authentication logic after receiving 401s during the `ghcr.io` continuous integration tasks.

#### v4.5.2
*   **Stale Filter Expiration logic**: Discovered that dynamic `?box=` payloads leveraging `serde_json` fallback for complex tier filters bypasses the manual `lat`/`lon` expiration drop rules. Substituted fallback implementation dynamically with `aircraft_json_t3` helper for correct viewport track expiration handling.

#### v4.5.3
*   **Frontend Javascript Timeout Regression**: The MapLibre & `readsb`/`tar1090` frontend systems rely on exact metric computations using relative duration values `seen` and `seen_pos`. 
*   Because `tier=1` naturally attempted to heavily prune redundant variables, local client timestamps would freeze instead of age. This permanently stuck planes to the map.
*   The API structures were reverted and patched to strictly ensure timeout attributes always broadcast identically to legacy tier 3 systems, allowing frontends to automatically clean map markers according to standard memory. Bounds `?box` filtering was heavily reinforced against ghost transmissions for aircraft tracking that aged to limits > 60 seconds.
