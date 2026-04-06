# Skylink - Aircraft Tracker

Production deployment for [skylink.hpradar.com](https://skylink.hpradar.com)

## Deploy

```bash
git clone git@github.com:ngtrthanh/skylink.git
cd skylink
cp .env.example .env  # edit feeder config
docker compose up -d
```

## Image

Pinned to exact digest in `docker-compose.yml`. To update:

```bash
docker pull ghcr.io/sdr-enthusiasts/docker-tar1090:latest
docker inspect ghcr.io/sdr-enthusiasts/docker-tar1090:latest --format '{{index .RepoDigests 0}}'
# Update the sha256 in docker-compose.yml
docker compose up -d
```

## Customizations

All in `local/skylink-lc2/` (bind-mounted as `/var/tar1090_git_source`):
- `html/config.js` — settings + receivers-overlay loader + myExtent clamp
- `html/receivers-overlay.js` — feeder map overlay (LayerSwitcher toggle)
- `nginx-perf.conf` — gzip tuning

**Rule**: never modify `script.js` or other cache-busted files directly. Use `config.js` for all customizations.

## Ports

| Port | Service |
|------|---------|
| 31787 | Web UI (CF tunnel) |
| 30004 | Beast input (feeders) |
| 30005/33005 | Beast output |
| 30006/32006 | BeastReduce output |
