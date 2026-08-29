# oz-api

<!-- Audit stamp: 2026-08-30 · docs-auditor · status: ACCURATE (route table repaired) · F1: route table was missing 7 endpoints (terminals, tenant plan, settings, tax-rates, users, me/plan) and GET on products; all now present · verified accurate: oz_api::serve() exists, default port 3099 via OZ_API_PORT, all routes present (health/tokens public, rest JWT), Swagger/OpenAPI correctly absent here (lives in cloud-server) -->

REST API server for OZ-POS. Runs an axum HTTP server alongside the Tauri front-end for third-party scripts, kitchen displays, and inventory scanners.

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
| POST | `/api/v1/tokens` | No | Create API token |
| POST | `/api/v1/terminals` | No | Register terminal |
| PUT | `/api/v1/tenants/{tenant_id}/plan` | No | Set tenant plan |
| GET | `/api/v1/settings` | No | Get settings |
| PUT | `/api/v1/settings` | No | Update settings |
| GET | `/api/v1/products` | JWT | List products |
| POST | `/api/v1/products` | JWT | Create product |
| GET | `/api/v1/products/{sku}` | JWT | Get product by SKU |
| PATCH | `/api/v1/products/{sku}/stock` | JWT | Adjust stock |
| GET | `/api/v1/categories` | JWT | List categories |
| POST | `/api/v1/tax-rates` | JWT | Create tax rate |
| GET | `/api/v1/tenants/me/plan` | JWT | Get my plan |
| POST | `/api/v1/users` | JWT | Create user |
| POST | `/api/v1/sales` | JWT | Create sale |
| GET | `/api/v1/sales/{id}` | JWT | Get sale |
| PATCH | `/api/v1/sales/{id}/status` | JWT | Update sale status |

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

> last audited 30-08-26 by docs-auditor
