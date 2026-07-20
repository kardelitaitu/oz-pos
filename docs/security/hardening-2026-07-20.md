# Input Validation & Rate Limiting — 0.0.14 Hardening

## Input Validation

All 250+ Tauri commands use Rust's type system for basic validation (strongly typed parameters prevent injection at the serialization layer).
Additional hardening recommendations:

### Critical Commands (spot-checked)

| Command | Validation | Status |
|---------|-----------|--------|
| `check_login` | Username: 1-100 chars, alphanumeric + `._-`. PIN: exactly 4-6 digits | ✅ `auth.rs` |
| `create_sale` | Cart validation: currency match, qty > 0, price ≥ 0. Payment amount ≤ total | ✅ `pos.rs` |
| `import_data` | JSON schema validation, foreign key checks, size limit | ✅ `data.rs` |
| `search_products` | Query length ≤ 200 chars, SQL injection prevented via parameterized queries | ✅ `products.rs` |
| `build_custom_report` | Column whitelist validation, parameterized date values | ✅ `export/mod.rs` |

### Guidelines for Future Commands

- **String inputs**: Max length check before DB query (prevents oversized payloads)
- **Numeric inputs**: Range validation (price ≥ 0, qty 1-9999)
- **File paths**: Resolve relative to app data dir (prevents path traversal)
- **SQL**: Always use `rusqlite` parameterized queries — never string interpolation
- **Tauri State**: Session token validation on scoped commands

## Rate Limiting

Already implemented in `apps/cloud-server/` (P8-1). Token-bucket algorithm with per-tenant per-endpoint buckets.

| Endpoint | Limit | Status |
|----------|-------|--------|
| `/api/v1/sync/push` | 100/min | ✅ |
| `/api/v1/sync/pull` | 300/min | ✅ |
| `/api/v1/sync/status` | 300/min | ✅ |
| `/api/v1/sync/snapshot` | 50/min | ✅ |
| All other `/api/*` | 300/min (default) | ✅ |

Middleware returns `429 Too Many Requests` with `Retry-After` header. Background cleanup (60s interval) removes stale buckets.
