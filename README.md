# Skylink - Aircraft Tracker

Production deployment for [skylink.hpradar.com](https://skylink.hpradar.com)

## Deploy

```bash
git clone git@github.com:ngtrthanh/skylink.git
cd skylink
docker compose up -d
```

## Update image

```bash
docker pull ghcr.io/sdr-enthusiasts/docker-tar1090:latest
docker inspect ghcr.io/sdr-enthusiasts/docker-tar1090:latest --format '{{index .RepoDigests 0}}'
# Update sha256 in docker-compose.yml, commit, tag, push
docker compose up -d
```

## Customizations

All in `local/skylink-lc2/` (mounted as `/var/tar1090_git_source`):
- `html/config.js` — settings, receivers-overlay loader, myExtent clamp
- `html/receivers-overlay.js` — feeder map overlay
- `nginx-perf.conf` — gzip tuning

**Never modify `script.js` directly** — use `config.js` for all customizations.

## Ports

| Port | Service |
|------|---------|
| 31787 | Web UI (CF tunnel) |
| 30004 | Beast input (feeders) |
| 33005 | Beast output |
| 32006 | BeastReduce output |
