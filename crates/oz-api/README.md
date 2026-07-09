# oz-api

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
