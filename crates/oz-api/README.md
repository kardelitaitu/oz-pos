# oz-api

<!-- Audit stamp: 2026-09-03 · DSH · status: ACCURATE (route table re-repaired + auth column corrected) · F1: table was again missing 10 routes (exchange-rates ×5, images ×5) · F2: auth column was wrong — tokens/terminals/plan/settings are admin-key-gated (X-Admin-Key when OZ_ADMIN_KEY set; open in dev), settings gated inside the handler despite public-router placement; master-data writes (products POST, stock PATCH, tax-rates POST, exchange-rates POST/DELETE, users POST) additionally require the admin key and reject terminal-scoped tokens (D1) · verified accurate: oz_api::serve() exists, default port 3099 via OZ_API_PORT, Swagger/OpenAPI correctly absent here (lives in cloud-server) · NOTE: serve() is not yet started by desktop/tablet — see docs/guides/EXTENDING.md §10 -->

REST API server for OZ-POS. An axum HTTP API for third-party scripts, kitchen displays, and inventory scanners. Mounted by `apps/cloud-server` today; intended to also run alongside the Tauri front-end (not wired yet — see [EXTENDING guide](../../docs/guides/EXTENDING.md) §10).

## Quick start

```rust
// Background task in apps/desktop-client/src/main.rs
oz_api::serve().await?;
```

Listens on `OZ_API_PORT` (default `3099`). DB path from `OZ_DB_PATH` (default `oz-pos.db`).

## API routes

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/health` | No | Health check |
| POST | `/api/v1/tokens` | Admin¹ | Mint JWT (admin-key path or terminal client-credentials path) |
| POST | `/api/v1/terminals` | Admin¹ | Register terminal; returns `device_secret` once, re-register rotates |
| PUT | `/api/v1/tenants/{tenant_id}/plan` | Admin¹ | Set tenant plan |
| GET | `/api/v1/settings` | Admin¹ | Read tenant effective settings (SMTP / report schedule) |
| PUT | `/api/v1/settings` | Admin¹ | Update tenant scoped settings |
| GET | `/api/v1/products` | JWT | List products |
| POST | `/api/v1/products` | JWT + Admin² | Create product |
| GET | `/api/v1/products/{sku}` | JWT | Get product by SKU |
| PATCH | `/api/v1/products/{sku}/stock` | JWT + Admin² | Adjust stock (signed delta) |
| GET | `/api/v1/categories` | JWT | List categories |
| POST | `/api/v1/tax-rates` | JWT + Admin² | Create tax rate |
| GET | `/api/v1/exchange-rates` | JWT | Full rate history |
| POST | `/api/v1/exchange-rates` | JWT + Admin² | Create rate (6-decimal fixed point) |
| GET | `/api/v1/exchange-rates/latest` | JWT | Newest rate per pair |
| GET | `/api/v1/exchange-rates/latest/{from}/{to}` | JWT | Newest rate for one pair |
| DELETE | `/api/v1/exchange-rates/{id}` | JWT + Admin² | Delete rate |
| GET | `/api/v1/tenants/me/plan` | JWT | Get my plan |
| POST | `/api/v1/users` | JWT + Admin² | Create user |
| POST | `/api/v1/sales` | JWT | Create sale |
| GET | `/api/v1/sales/{id}` | JWT | Get sale |
| PATCH | `/api/v1/sales/{id}/status` | JWT | Update sale status |
| PUT | `/api/v1/images` | JWT | Store one WebP (≤ 32 KB, content-addressed) |
| POST | `/api/v1/images` | JWT | Batch store (≤ 16 files / 512 KB, length-prefixed frames) |
| GET | `/api/v1/images:pack` | JWT | Fetch ≤ 64 files as binary frames |
| GET | `/api/v1/images:missing` | JWT | Missing-hash set difference |
| GET | `/api/v1/images/{hash16}` | JWT | Immutable WebP bytes |

¹ `X-Admin-Key` header required when `OZ_ADMIN_KEY` is configured; open in dev mode.
² Operator write tier (D1): admin key **and** a non-terminal token — device credentials
must never mutate master data. Sales writes are exempt (terminals sell).
GETs on JWT routes are additionally gated by read-tier permissions (spec 0047).
Full contract, auth model, and recipes: [docs/guides/EXTENDING.md](../../docs/guides/EXTENDING.md).

```bash
# Generate token
curl -X POST http://localhost:3099/api/v1/tokens \
  -H "Content-Type: application/json" \
  -d '{"label": "my-script"}'

# Use token
curl http://localhost:3099/api/v1/products \
  -H "Authorization: Bearer <token>"
```

## State

`AppState` wraps SQLite in `Arc<Mutex<Connection>>`. CORS uses a configurable origin allowlist (`OZ_CORS_ORIGINS`, default `DEFAULT_CORS_ORIGINS`; `"*"` is an explicit dev opt-in, otherwise fail-closed). All JWT-protected routes return 401 without a valid token.

> last audited 03-09-26 by DSH
