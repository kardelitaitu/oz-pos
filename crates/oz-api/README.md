# oz-api

<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (1 noted finding — incomplete, not wrong) · F1: route table omits 2 now-existing endpoints present in lib.rs: POST /api/v1/tax-rates (tax_rates::create_tax_rate) and POST /api/v1/users (users::create_user) · verified accurate: oz_api::serve() exists, default port 3099 via OZ_API_PORT, all 10 listed routes present (health/tokens public, rest JWT), Swagger/OpenAPI correctly absent here (lives in cloud-server) -->

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
| GET | `/api/v1/products` | JWT | List products |
| POST | `/api/v1/products` | JWT | Create product |
| GET | `/api/v1/products/{sku}` | JWT | Get product by SKU |
| PATCH | `/api/v1/products/{sku}/stock` | JWT | Adjust stock |
| GET | `/api/v1/categories` | JWT | List categories |
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

`AppState` wraps SQLite in `Arc<Mutex<Connection>>`. CORS allows any origin. All JWT-protected routes return 401 without a valid token.

> last audited 28-06-26 by docs-auditor
