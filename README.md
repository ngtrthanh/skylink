# Skylink

ADS-B + AIS aggregator for [skylink.hpradar.com](https://skylink.hpradar.com)

## Structure

```
about/                — project memos, API docs, architecture decisions
input/                — reference repos (readsb, AIS-catcher, tar1090-fe) — gitignored
output/
  skylink-core/       — Rust backend (the product)
  skylink-fe/         — MapLibre frontend
  ml_clf_fe/          — ML classifier frontend
  deploy-template/    — compose.yaml + .env.example (copy to deploy host)
```

## Workspace Layout (outside repo)

```
/opt/workspace/
  dev/hpradar.com/skylink/         — this repo (development)
  deploy/hpradar.com/skylink/      — production deployment
  staging/hpradar.com/skylink/     — staging deployment
  sandbox/hpradar.com/skylink/     — test runs & experiments
```

## Quick Start

```bash
# Production / Staging: copy template to deploy host
cp -r output/deploy-template/ /opt/workspace/deploy/hpradar.com/skylink/
cd /opt/workspace/deploy/hpradar.com/skylink/
cp .env.example .env   # edit as needed
docker compose up -d
```

## Docs

- [API Reference](about/API.md)
- [v4.1 Recap](about/MEMO-v4-recap.md)
