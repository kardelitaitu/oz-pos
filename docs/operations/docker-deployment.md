# Docker Deployment Guide — Full-Stack OZ-POS

> **ADR:** [ADR #11](../decisions/2026-07-13-zero-downtime-vps-migration.md)
> **Status:** Implemented (2026-07-20)
> **Target audience:** DevOps / system administrators

This guide covers deploying the complete OZ-POS backend stack using Docker
Compose: cloud server, license server, Redis cache, and optional PostgreSQL.

---

## Architecture Overview

```
                   ┌─────────────────────────────────────┐
                   │         Docker Compose (one host)     │
                   │                                       │
  POS Terminal ───►│  pos-cloud-server  (port 3099)       │
                   │        │                              │
                   │        ├──► redis (cache + pub/sub)   │
                   │        │                              │
                   │        └──► pos-cloud-db (PostgreSQL) │
                   │                     (optional, PG)    │
                   │                                       │
  POS Terminal ───►│  license-server    (port 8080)        │
                   │         │                              │
                   │         └──► (embedded SQLite)         │
                   └─────────────────────────────────────┘
```

### Service Summary

| Service | Language | Port | Purpose | DB |
|---------|----------|------|---------|----|
| `pos-cloud-server` | Rust | 3099 | Sync API, auth, webhooks | SQLite or PostgreSQL |
| `license-server` | Go | 8080 | License activation, renewal | Embedded SQLite (PocketBase) |
| `redis` | — | 6379 | Product cache, inventory pub/sub | In-memory (persistent RDB) |
| `pos-cloud-db` | — | 5432 | Enterprise database backend | PostgreSQL 16 (optional) |

### Port Map

| Port | Service | Protocol | Notes |
|------|---------|----------|-------|
| `3099` | pos-cloud-server | HTTP | Sync API + health endpoint |
| `8080` | license-server | HTTP | License API + PocketBase admin UI (`/_/`) |
| `6379` | redis | TCP | Redis protocol (internal only in production) |
| `5432` | pos-cloud-db | TCP | PostgreSQL protocol (internal only in production) |

---

## Quick Start (SQLite)

The fastest way to get the full stack running with no external dependencies:

```bash
# 1. Generate license keys (one time)
bash scripts/generate-license-keys.sh        # Linux/macOS
powershell -File scripts/generate-license-keys.ps1   # Windows

# 2. Start all services
docker compose up -d

# 3. Verify health
curl http://localhost:3099/api/v1/health
curl http://localhost:8080/api/v1/license/status

# 4. Create admin user for license server
docker compose exec license-server \
  /pb/pocketbase superuser upsert admin@example.com password123

# 5. View logs
docker compose logs -f
```

The cloud server starts with SQLite (default) and connects to Redis at
`redis://redis:6379`. The license server uses embedded PocketBase SQLite.

### Step-by-Step

1. **Generate license signing keys** — These are required by the license
   server to sign subscription tokens. The script saves the private key
   to `crates/oz-core/oz-license-private.pem`.

2. **Start the stack** — `docker compose up -d` starts `pos-cloud-server`,
   `license-server`, and `redis` in the default profile. The cloud server
   waits for Redis to be healthy before starting. The license server starts
   immediately (no external dependencies).

3. **Create admin user** — The PocketBase admin UI at `http://localhost:8080/_/`
   needs a superuser. Use the `docker compose exec` command above.

4. **Health check** — Both services expose health endpoints:

   | Service | Endpoint | Response |
   |---------|----------|----------|
   | Cloud server | `GET /api/v1/health` | `{"status":"ok","version":"0.0.13",...}` |
   | License server | `GET /api/health` | `{"status":"ok","uptime_seconds":...}` |

5. **Test license activation** — Once running, activate a license:

   ```bash
   curl -X POST http://localhost:8080/api/v1/license/activate \
     -H "Content-Type: application/json" \
     -d '{"key":"OZ-PRO-TEST-ABCD-EFGH-IJKL","tenant_id":"t1","machine_id":"m1"}'
   ```

---

## With PostgreSQL (Enterprise)

For production deployments requiring PostgreSQL instead of SQLite:

```bash
# 1. Generate keys
bash scripts/generate-license-keys.sh

# 2. Set required env vars
export OZ_API_SECRET=$(openssl rand -hex 32)

# 3. Start with pg profile
docker compose --profile pg up -d

# 4. Verify all services healthy
curl http://localhost:3099/api/v1/health
curl http://localhost:8080/api/v1/license/status
```

Environment variables for PostgreSQL:

| Variable | Default | Description |
|----------|---------|-------------|
| `PG_USER` | `ozpos` | PostgreSQL user |
| `PG_PASSWORD` | `changeme` | PostgreSQL password |
| `PG_DATABASE` | `ozpos` | PostgreSQL database name |
| `DATABASE_URL` | *(auto-constructed)* | Full connection string |

When the `pg` profile is active, the cloud server waits for PostgreSQL
to be healthy before starting (`depends_on: pos-cloud-db: condition: service_healthy`).

---

## Environment Variables Reference

### Required in Production

| Variable | Default | Service | Description |
|----------|---------|---------|-------------|
| `OZ_API_SECRET` | _(empty)_ | pos-cloud-server | JWT signing secret. Generate: `openssl rand -hex 32` |
| `OZ_LICENSE_PRIVATE_KEY` | _(empty)_ | license-server | PEM-encoded license signing private key |

### Optional

| Variable | Default | Service | Description |
|----------|---------|---------|-------------|
| `OZ_API_PORT` | `3099` | pos-cloud-server | HTTP listen port |
| `RUST_LOG` | `info` | pos-cloud-server | Log level (debug, info, warn, error) |
| `OZ_DB_PATH` | `/data/oz-pos.db` | pos-cloud-server | SQLite database path |
| `REDIS_URL` | `redis://redis:6379` | pos-cloud-server | Redis connection string |
| `REDIS_CACHE_TTL` | `300` | pos-cloud-server | Redis cache TTL (seconds) |
| `DATABASE_URL` | _(empty)_ | pos-cloud-server | PostgreSQL connection string |
| `PG_USER` | `ozpos` | pos-cloud-db | PostgreSQL user |
| `PG_PASSWORD` | `changeme` | pos-cloud-db | PostgreSQL password |
| `PG_DATABASE` | `ozpos` | pos-cloud-db | PostgreSQL database name |

---

## Healthcheck Dependencies

The docker-compose.yml defines the following healthcheck chain:

```
redis (healthy)
  │
  └──► pos-cloud-server (waits for redis + optional PG)
         │
         └──► pos-cloud-db (only when pg profile active, required: false)
               │
               └──► depends_on: condition: service_healthy

license-server (no dependencies — standalone)
```

Key design decisions:
- **`depends_on` with `condition: service_healthy`** — The cloud server
  waits for Redis (and PostgreSQL when active) to pass healthchecks before
  starting. This prevents startup race conditions.
- **`required: false` on `pos-cloud-db`** — The pg profile is optional.
  When inactive, the dependency is ignored and the cloud server uses SQLite.
- **`license-server` has no `depends_on`** — It uses embedded SQLite
  (PocketBase) and starts immediately.

### Healthcheck Details

| Service | Test Command | Interval | Timeout | Retries | Start Period |
|---------|-------------|----------|---------|---------|-------------|
| pos-cloud-server | `wget --spider /api/v1/health` | 15s | 5s | 3 | 30s |
| license-server | `/pb/healthcheck /api/health` | 15s | 5s | 3 | 10s |
| redis | `redis-cli ping` | 10s | 3s | 5 | 5s |
| pos-cloud-db | `pg_isready -U ozpos -d ozpos` | 10s | 5s | 5 | 15s |

---

## Volume Management

| Volume | Service | Mount Point | Purpose |
|--------|---------|-------------|---------|
| `oz_cloud_data` | pos-cloud-server | `/data` | SQLite database + WAL files |
| `pb_data` | license-server | `/pb/pb_data` | PocketBase embedded SQLite |
| `redis_data` | redis | `/data` | Redis RDB persistence |
| `oz_pg_data` | pos-cloud-db | `/var/lib/postgresql/data` | PostgreSQL data directory |

### Backup Commands

```bash
# Backup SQLite database
docker run --rm -v oz_cloud_data:/data -v $(pwd):/backup alpine \
  cp /data/oz-pos.db /backup/oz-pos-$(date +%Y%m%d).db

# Backup PocketBase database
docker run --rm -v pb_data:/pb -v $(pwd):/backup alpine \
  cp /pb/pb_data/data.db /backup/pb-data-$(date +%Y%m%d).db
```

### Cleanup

```bash
# Stop and remove containers (preserves volumes)
docker compose down

# Stop and remove everything (DESTROYS DATA)
docker compose down -v
```

---

## Networking

All services are on the default Compose network (`oz-pos_default`). They
resolve each other by service name:

| Service | Internal hostname | Port |
|---------|-------------------|------|
| pos-cloud-server | `pos-cloud-server` | 3099 |
| license-server | `license-server` | 8080 |
| redis | `redis` | 6379 |
| pos-cloud-db | `pos-cloud-db` | 5432 |

### Internal Communication Examples

- Cloud server → Redis: `redis://redis:6379`
- Cloud server → PostgreSQL: `postgresql://ozpos:changeme@pos-cloud-db:5432/ozpos`
- Terminal → Cloud server: `http://pos-cloud-server:3099` (within Docker network)
- Terminal → License server: `http://license-server:8080` (within Docker network)

---

## Logs & Monitoring

```bash
# All services
docker compose logs -f

# Single service
docker compose logs -f pos-cloud-server
docker compose logs -f license-server

# Tail last 100 lines
docker compose logs --tail=100 pos-cloud-server
```

---

## Common Operations

### Add a License

```bash
# Create a license in PocketBase
curl -X POST http://localhost:8080/api/v1/license/activate \
  -H "Content-Type: application/json" \
  -d '{"key":"OZ-PRO-ABCD-EFGH-IJKL","tenant_id":"tenant_1","machine_id":"pos-01"}'
```

### View Redis Cache Stats

```bash
docker compose exec redis redis-cli INFO stats
```

### Run Migrations Manually

```bash
docker compose exec pos-cloud-server /app/oz-cloud-server --migrate
```

---

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `pos-cloud-server` exits immediately | Missing `OZ_API_SECRET` | Set the env var |
| `license-server` exits immediately | Missing `OZ_LICENSE_PRIVATE_KEY` | Run `scripts/generate-license-keys.*` and set the env var |
| Cloud server can't connect to Redis | Redis not healthy yet | Check `docker compose logs redis`; wait for healthcheck |
| `redis-cli ping` fails | Redis not responding | `docker compose restart redis` |
| License activation returns 401 | Wrong or missing private key | Regenerate keys |
| Port 3099 already in use | Another service using the port | Set `OZ_API_PORT=3100` and update firewall |
| Port 8080 already in use | Another service using the port | Edit `docker-compose.yml` port mapping |
| Can't access PocketBase admin UI | No superuser created | Run the `superuser upsert` command |
| Slow product lookups | Redis cache cold | Wait for cache to warm up (first requests are slower) |
| `docker compose --profile pg up` fails | PG profile depends on `pos-cloud-db` | Ensure `redis` is also running (no profile needed) |

---

## Security Notes

1. **Change default passwords** — The PostgreSQL default credentials
   (`ozpos` / `changeme`) are for development only. Set `PG_PASSWORD`
   in production.

2. **Do not expose Redis or PostgreSQL ports externally** — Set
   `ports: ["6379:6379"]` only for local development. In production,
   remove the port mapping or bind to `127.0.0.1`.

3. **Use secrets for sensitive env vars** — In production, prefer
   Docker secrets or your orchestrator's secret store instead of
   plain env vars:

   ```yaml
   secrets:
     oz_api_secret:
       file: ./secrets/oz_api_secret.txt

   services:
     pos-cloud-server:
       secrets:
         - oz_api_secret
       environment:
         OZ_API_SECRET_FILE: /run/secrets/oz_api_secret
   ```

4. **TLS termination** — These services speak HTTP. Use a reverse proxy
   (nginx, Caddy, Traefik) in front for TLS termination in production.

5. **License private key** — Treat `oz-license-private.pem` as a
   critical secret. Store it in a password manager or secrets vault.
   If compromised, all existing licenses are invalidated.

---

## Related

- [VPS Migration Guide](./vps-migration.md) — Zero-downtime server migration
- [ADR #11: VPS Migration Strategy](../decisions/2026-07-13-zero-downtime-vps-migration.md)
- [ADR #10: Sync Performance Strategy](../decisions/2026-07-13-sync-performance-compression-batching.md)
- [`Dockerfile.server`](../../Dockerfile.server) — Cloud server Docker build
- [`apps/license-server/Dockerfile`](../../apps/license-server/Dockerfile) — License server Docker build
- [`scripts/generate-license-keys.sh`](../../scripts/generate-license-keys.sh) — License key generation
