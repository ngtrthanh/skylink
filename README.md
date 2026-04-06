# Skylink - Aircraft Tracker

Production deployment for [skylink.hpradar.com](https://skylink.hpradar.com)

## Deploy

```bash
git clone git@github.com:ngtrthanh/skylink.git
cd skylink
docker compose up -d
```

## Update

Build new image from running container:
```bash
docker commit skylink ghcr.io/ngtrthanh/skylink:vX.Y.Z
docker push ghcr.io/ngtrthanh/skylink:vX.Y.Z
# Update sha256 in docker-compose.yml
```

## Ports

| Port | Service |
|------|---------|
| 31787 | Web UI (CF tunnel) |
| 30004 | Beast input (feeders) |
| 33005 | Beast output |
| 32006 | BeastReduce output |
