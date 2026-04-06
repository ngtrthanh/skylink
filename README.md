# Skylink - Aircraft Tracker

Production deployment for [skylink.hpradar.com](https://skylink.hpradar.com)

## CI/CD

Push a tag → GitHub Actions SSHs into server → pulls + restarts.

### Setup (one-time)

Add these GitHub repo secrets (`Settings > Secrets > Actions`):
- `SERVER_HOST` — server IP or hostname
- `SERVER_USER` — SSH user
- `SERVER_SSH_KEY` — private SSH key

### Deploy workflow

```bash
# 1. Make changes to docker-compose.yml (e.g. new image digest)
# 2. Commit
git add -A && git commit -m "v1.2.0: description"

# 3. Tag and push — triggers deploy
git tag -a v1.2.0 -m "description"
git push origin main --tags
```

GitHub Actions will:
1. SSH into server
2. `git checkout v1.2.0`
3. `docker compose pull && up -d`
4. Health check (45s timeout)

### Update image

```bash
# On server: commit running container as new image
docker commit skylink ghcr.io/ngtrthanh/skylink:v1.2.0
docker push ghcr.io/ngtrthanh/skylink:v1.2.0

# Get digest
docker inspect ghcr.io/ngtrthanh/skylink:v1.2.0 --format '{{index .RepoDigests 0}}'

# Update docker-compose.yml with new sha256, commit, tag, push
```

### Rollback

```bash
git checkout v1.1.0
docker compose up -d --force-recreate
```

## Ports

| Port | Service |
|------|---------|
| 31787 | Web UI (CF tunnel) |
| 30004 | Beast input (feeders) |
| 33005 | Beast output |
| 32006 | BeastReduce output |
