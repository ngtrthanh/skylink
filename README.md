# Skylink

ADS-B + AIS aggregator for [skylink.hpradar.com](https://skylink.hpradar.com)

## Structure

```
about/          — project memos, API docs, architecture decisions
input/          — reference repos (readsb, AIS-catcher, tar1090-fe) — gitignored
output/         — frontends (skylink-fe MapLibre app)
skylink-core/   — Rust backend (the product)
deploy/         — production compose + env config
```

## Quick Start

```bash
cd deploy
cp .env.example .env    # edit as needed
docker compose up -d
```

## Docs

- [API Reference](about/API.md)
- [v4.1 Recap](about/MEMO-v4-recap.md)
