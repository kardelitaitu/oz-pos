//! OpenAPI 3.1 API documentation for the OZ-POS cloud server.
//!
//! Serves:
//! - `GET /api/openapi.json` — the OpenAPI 3.1 specification
//! - `GET /api/docs` — Swagger UI (loaded from CDN) pointing at the spec
//! - `GET /api/docs/scalar` — Scalar API Reference (modern, interactive docs)
//!
//! The spec is generated programmatically from schema builders for
//! maintainability — no external OpenAPI crate dependency required.

use axum::{Json, response::Html};
use serde_json::{Value, json};

/// Returns the OpenAPI 3.1 specification as a JSON value.
///
/// This documents all 20 endpoints across 7 tag groups: Health, Auth,
/// Products, Categories, Tax Rates, Users, Sales, Sync, and Webhooks.
pub fn openapi_spec() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "OZ-POS Cloud Server API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "REST API for the OZ-POS point-of-sale cloud sync server.\n\n## Authentication\nMost endpoints require a JWT bearer token from `POST /api/v1/tokens`. Pass it as `Authorization: Bearer <token>`.\n\n## Versioning\nThe API is versioned by URL path prefix (`/api/v1/`). Breaking changes will ship under a new version prefix (`/api/v2/`) — the old version remains available for at least 6 months after the new one lands.\n\n## Pagination\nList endpoints accept `?limit` (default 50, max 200) and `?offset` (default 0) query parameters and return a `PaginatedResponse` envelope with `data`, `total`, `limit`, and `offset` fields.\n\n## Errors\nAll error responses share a common envelope: `{ \"error\": { \"code\": \"MACHINE_READABLE\", \"message\": \"Human description\", \"details\": [...] } }`. The `code` field is stable across versions — use it for programmatic error handling, not the message string.\n\n## Rate Limiting\nSync endpoints return `X-RateLimit-Remaining`, `X-RateLimit-Reset`, and `Retry-After` headers when nearing the per-tenant limit.",
            "contact": { "name": "OZ-POS" }
        },
        "servers": [
            { "url": "http://localhost:{port}", "description": "Local development server", "variables": { "port": { "default": "3099", "description": "Server port (OZ_API_PORT env var)" } } },
            { "url": "https://{host}", "description": "Production server (behind reverse proxy)", "variables": { "host": { "default": "pos.example.com", "description": "Your deployment hostname" } } }
        ],
        "externalDocs": {
            "description": "OZ-POS documentation",
            "url": "https://github.com/oz-pos/oz-pos"
        },
        "tags": [
            { "name": "Health", "description": "Server health and monitoring endpoints" },
            { "name": "Auth", "description": "Token generation and authentication" },
            { "name": "Products", "description": "Product CRUD and stock management" },
            { "name": "Categories", "description": "Product category listing" },
            { "name": "Tax Rates", "description": "Tax rate configuration" },
            { "name": "Users", "description": "User account management" },
            { "name": "Sales", "description": "Sale creation, retrieval, and status transitions" },
            { "name": "Sync", "description": "Offline queue push/pull sync endpoints" },
            { "name": "Plans", "description": "Tenant cloud sync plans (ADR sync-plan-gating)" },
            { "name": "Terminals", "description": "Terminal registration for client-credential authentication" },
            { "name": "Webhooks", "description": "Third-party payment provider webhook receivers" },
            { "name": "Inventory", "description": "Stock movements, transfers, low-stock alerts, and purchase order management" },
            { "name": "Orders", "description": "Kitchen display order routing, course firing, and production tracking" },
            { "name": "Reports", "description": "Sales summaries, category breakdowns, hourly heatmaps, and staff performance reports" },
            { "name": "Customers", "description": "Customer profiles, loyalty points, gift card balances, and CRM integrations" },
            { "name": "Notifications", "description": "Push notification registration, email alerts, and in-app messaging" },
            { "name": "Analytics", "description": "Menu engineering scores, popularity metrics, trend forecasts, and margin analysis" }
        ],
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT",
                    "description": "JWT token obtained from `POST /api/v1/tokens`. Include as `Authorization: Bearer <token>`."
                }
            },
            "schemas": build_schemas()
        },
        "paths": build_paths()
    })
}

/// Returns a Swagger UI HTML page that loads the spec from `/api/openapi.json`.
///
/// Uses the unpkg CDN for Swagger UI assets. No additional dependencies needed.
pub fn swagger_ui_html() -> Html<String> {
    Html(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>OZ-POS API Docs — Swagger UI</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
    <style>
        html {{ box-sizing: border-box; overflow-y: scroll; }}
        *, *::before, *::after {{ box-sizing: inherit; }}
        body {{ margin: 0; background: #fafafa; }}
        .topbar {{ display: none; }}
        .swagger-ui .info {{ margin: 20px 0; }}
        .swagger-ui .info .title {{ font-size: 28px; }}
        .swagger-ui .scheme-container {{ display: none; }}
        .version-badge {{
            display: inline-block;
            background: #49cc90;
            color: #fff;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 13px;
            margin-left: 8px;
            vertical-align: middle;
        }}
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js" crossorigin></script>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-standalone-preset.js" crossorigin></script>
    <script>
        window.onload = function() {{
            window.ui = SwaggerUIBundle({{
                url: "/api/openapi.json",
                dom_id: "#swagger-ui",
                deepLinking: true,
                presets: [SwaggerUIBundle.presets.apis, SwaggerUIStandalonePreset],
                plugins: [SwaggerUIBundle.plugins.DownloadUrl],
                layout: "StandaloneLayout",
                defaultModelsExpandDepth: 1,
                defaultModelExpandDepth: 1,
                docExpansion: "list",
                filter: true,
                showExtensions: true,
                showCommonExtensions: true,
                tryItOutEnabled: true,
            }});
        }};
    </script>
</body>
</html>"##.to_string()
    )
}

/// Returns a Scalar API Reference HTML page that loads the spec from `/api/openapi.json`.
///
/// Scalar is a modern, interactive API documentation UI with a clean design.
/// Loaded from CDN — no additional dependencies needed.
pub fn scalar_html() -> Html<String> {
    Html(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>OZ-POS API Docs — Scalar</title>
    <style>
        body { margin: 0; padding: 0; }
    </style>
</head>
<body>
    <script
        id="api-reference"
        data-url="/api/openapi.json"
        data-proxy-url="https://proxy.scalar.com">
    </script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"##
            .to_string(),
    )
}

// ── Schema builders ────────────────────────────────────────────────────

fn build_schemas() -> Value {
    json!({
        "ErrorResponse": {
            "type": "object",
            "required": ["error"],
            "properties": {
                "error": { "type": "string", "description": "Human-readable error description" }
            },
            "deprecated": true,
            "description": "Legacy flat error format. New code should use `ErrorEnvelope`."
        },
        "ErrorEnvelope": {
            "type": "object",
            "required": ["error"],
            "properties": {
                "error": {
                    "type": "object",
                    "required": ["code", "message"],
                    "properties": {
                        "code": { "type": "string", "description": "Machine-readable error code (stable across versions)", "example": "invalid_status_transition" },
                        "message": { "type": "string", "description": "Human-readable description", "example": "Cannot transition from pending to completed" },
                        "details": { "type": "array", "items": { "type": "object" }, "description": "Optional per-field validation details" }
                    }
                }
            },
            "description": "Standard error envelope for all 4xx/5xx responses. The `code` field is the stable contract — use it for programmatic handling."
        },
        "PaginatedResponse": {
            "type": "object",
            "required": ["data", "total", "limit", "offset"],
            "properties": {
                "data": { "type": "array", "items": {}, "description": "Page of results (item schema varies by endpoint)" },
                "total": { "type": "integer", "format": "int64", "description": "Total number of items matching the query (across all pages)" },
                "limit": { "type": "integer", "format": "int64", "description": "Requested page size (max 200)" },
                "offset": { "type": "integer", "format": "int64", "description": "Zero-based offset of the current page" }
            },
            "description": "Standard pagination envelope. All list endpoints will adopt this when pagination support lands."
        },
        "PaginationParams": {
            "limit": {
                "name": "limit",
                "in": "query",
                "required": false,
                "schema": { "type": "integer", "format": "int64", "default": 50, "maximum": 200 },
                "description": "Maximum items per page (default 50, max 200)"
            },
            "offset": {
                "name": "offset",
                "in": "query",
                "required": false,
                "schema": { "type": "integer", "format": "int64", "default": 0 },
                "description": "Zero-based page offset"
            },
            "sort": {
                "name": "sort",
                "in": "query",
                "required": false,
                "schema": { "type": "string" },
                "description": "Field to sort by (endpoint-specific)"
            },
            "order": {
                "name": "order",
                "in": "query",
                "required": false,
                "schema": { "type": "string", "enum": ["asc", "desc"], "default": "asc" },
                "description": "Sort order"
            },
            "q": {
                "name": "q",
                "in": "query",
                "required": false,
                "schema": { "type": "string" },
                "description": "Free-text search across name/SKU/barcode fields"
            }
        },
        "Money": {
            "type": "object",
            "required": ["minor_units", "currency"],
            "properties": {
                "minor_units": { "type": "integer", "format": "int64", "description": "Amount in minor units (e.g., 199 = $1.99)", "example": 199 },
                "currency": { "type": "string", "description": "ISO 4217 currency code", "example": "USD" }
            }
        },
        "HealthResponse": {
            "type": "object",
            "properties": {
                "status": { "type": "string", "description": "Server status: 'ok' or 'degraded'", "example": "ok" },
                "version": { "type": "string", "description": "Server version", "example": env!("CARGO_PKG_VERSION") },
                "db": { "type": "string", "description": "Database backend type", "example": "sqlite" },
                "uptime_seconds": { "type": "integer", "format": "int64", "description": "Seconds since server start" },
                "db_connected": { "type": "boolean", "description": "Whether the database responded to a ping" },
                "db_latency_us": { "type": "integer", "format": "int64", "description": "Database ping latency in microseconds" },
                "sync_queue_depth": { "type": "integer", "format": "int64", "description": "Number of pending items in the sync queue" },
                "last_sync_at": { "type": ["string", "null"], "description": "ISO-8601 timestamp of most recent sync" }
            }
        },
        "CreateTokenRequest": {
            "type": "object",
            "required": ["label"],
            "properties": {
                "label": { "type": "string", "description": "Human-readable label for the token", "example": "kitchen-display-1" },
                "expiry_hours": { "type": "integer", "format": "int64", "description": "Expiry in hours (default: 24)", "example": 24 },
                "tenant_id": { "type": "string", "description": "Optional tenant/store ID for multi-tenant isolation" }
            }
        },
        "CreateProductRequest": {
            "type": "object",
            "required": ["sku", "name", "price"],
            "properties": {
                "sku": { "type": "string", "description": "Unique product SKU", "example": "COFFEE-001" },
                "name": { "type": "string", "description": "Display name", "example": "Espresso" },
                "price": { "$ref": "#/components/schemas/Money" },
                "category_id": { "type": "string", "description": "Optional category ID" },
                "barcode": { "type": "string", "description": "Optional barcode (EAN-13, UPC-A, etc.)" },
                "initial_stock": { "type": "integer", "format": "int64", "description": "Initial stock quantity (0 or omitted = no inventory row)", "default": 0 }
            }
        },
        "ProductDetail": {
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Internal product ID" },
                "sku": { "type": "string" },
                "name": { "type": "string" },
                "price": { "$ref": "#/components/schemas/Money" },
                "category_id": { "type": ["string", "null"] },
                "category_name": { "type": ["string", "null"] },
                "barcode": { "type": ["string", "null"] },
                "stock_qty": { "type": ["integer", "null"], "format": "int64" },
                "created_at": { "type": "string", "format": "date-time" },
                "updated_at": { "type": "string", "format": "date-time" }
            }
        },
        "PatchStockRequest": {
            "type": "object",
            "required": ["delta"],
            "properties": {
                "delta": { "type": "integer", "format": "int64", "description": "Positive to restock, negative to sell", "example": -10 }
            }
        },
        "PatchStockResponse": {
            "type": "object",
            "properties": {
                "sku": { "type": "string" },
                "previous_qty": { "type": "integer", "format": "int64" },
                "new_qty": { "type": "integer", "format": "int64" }
            }
        },
        "CategoryDto": {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "name": { "type": "string", "example": "Drinks" },
                "colour": { "type": "string", "description": "Hex colour code", "example": "#06b6d4" },
                "created_at": { "type": "string", "format": "date-time" }
            }
        },
        "CreateTaxRateRequest": {
            "type": "object",
            "required": ["name", "rate_bps", "is_default", "is_inclusive"],
            "properties": {
                "name": { "type": "string", "description": "Display name", "example": "VAT 10%" },
                "rate_bps": { "type": "integer", "format": "int64", "description": "Rate in basis points (1000 = 10%)", "example": 1000 },
                "is_default": { "type": "boolean", "description": "Whether this is the default rate" },
                "is_inclusive": { "type": "boolean", "description": "Whether the rate is tax-inclusive" }
            }
        },
        "CreateUserRequest": {
            "type": "object",
            "required": ["username", "pin_hash", "display_name", "role_id"],
            "properties": {
                "username": { "type": "string", "description": "Unique username for login" },
                "pin_hash": { "type": "string", "description": "PIN hash for authentication" },
                "display_name": { "type": "string", "description": "Display name shown in the UI" },
                "role_id": { "type": "string", "description": "Role ID (must reference an existing role)", "example": "role-staff" }
            }
        },
        "SaleLineItem": {
            "type": "object",
            "required": ["sku", "qty", "unit_price"],
            "properties": {
                "sku": { "type": "string" },
                "qty": { "type": "integer", "format": "int64" },
                "unit_price": { "$ref": "#/components/schemas/Money" }
            }
        },
        "CreateSaleRequest": {
            "type": "object",
            "required": ["lines"],
            "properties": {
                "lines": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/SaleLineItem" },
                    "minItems": 1,
                    "description": "Line items (at least one required)"
                }
            }
        },
        "UpdateSaleStatusRequest": {
            "type": "object",
            "required": ["status"],
            "properties": {
                "status": { "type": "string", "description": "New status: 'pending', 'active', 'completed', or 'voided'", "enum": ["pending", "active", "completed", "voided"] }
            }
        },
        "SyncStatusResponse": {
            "type": "object",
            "properties": {
                "pending_count": { "type": "integer", "format": "int64" },
                "conflict_count": { "type": "integer", "format": "int64" },
                "total_items": { "type": "integer", "format": "int64" }
            }
        },
        "TokenResponse": {
            "type": "object",
            "required": ["token", "expires_at", "token_id"],
            "properties": {
                "token": { "type": "string", "description": "Signed JWT string", "example": "eyJhbGciOiJIUzI1NiJ9.eyJ..." },
                "expires_at": { "type": "string", "format": "date-time", "description": "ISO-8601 expiry timestamp" },
                "token_id": { "type": "string", "format": "uuid", "description": "Unique token identifier" }
            }
        },
        "CreateTokenResponse": {
            "type": "object",
            "required": ["token"],
            "properties": {
                "token": { "$ref": "#/components/schemas/TokenResponse" }
            }
        },
        "TaxRateResponse": {
            "type": "object",
            "required": ["id", "name", "rate_bps", "is_default", "is_inclusive"],
            "properties": {
                "id": { "type": "string", "description": "Unique tax rate ID" },
                "name": { "type": "string", "description": "Display name", "example": "VAT 10%" },
                "rate_bps": { "type": "integer", "format": "int64", "description": "Rate in basis points", "example": 1000 },
                "is_default": { "type": "boolean", "description": "Whether this is the default rate" },
                "is_inclusive": { "type": "boolean", "description": "Whether tax is inclusive of price" },
                "tenant_id": { "type": "string", "description": "Owning tenant (from JWT)" },
                "created_at": { "type": "string", "format": "date-time" }
            }
        },
        "UserResponse": {
            "type": "object",
            "required": ["id", "username", "display_name", "role_id"],
            "properties": {
                "id": { "type": "string", "description": "Unique user ID" },
                "username": { "type": "string", "description": "Login username" },
                "display_name": { "type": "string", "description": "Display name in UI" },
                "role_id": { "type": "string", "description": "Assigned role ID", "example": "role-staff" },
                "tenant_id": { "type": "string", "description": "Owning tenant (from JWT)" },
                "created_at": { "type": "string", "format": "date-time" }
            }
        },
        "SaleDetail": {
            "type": "object",
            "required": ["id", "status", "lines", "total", "created_at"],
            "properties": {
                "id": { "type": "string", "description": "Unique sale ID" },
                "status": { "type": "string", "enum": ["pending", "active", "completed", "voided"], "description": "Current sale status" },
                "lines": { "type": "array", "items": { "$ref": "#/components/schemas/SaleLineItem" }, "description": "Line items with computed line_total" },
                "total": { "$ref": "#/components/schemas/Money" },
                "subtotal": { "$ref": "#/components/schemas/Money" },
                "tax_total": { "$ref": "#/components/schemas/Money" },
                "created_at": { "type": "string", "format": "date-time" },
                "updated_at": { "type": "string", "format": "date-time" }
            }
        },
        "SaleStatusResponse": {
            "type": "object",
            "required": ["id", "status", "updated_at"],
            "properties": {
                "id": { "type": "string", "description": "Sale ID" },
                "status": { "type": "string", "enum": ["pending", "active", "completed", "voided"], "description": "Updated sale status" },
                "updated_at": { "type": "string", "format": "date-time", "description": "ISO-8601 timestamp of the update" }
            }
        },
        "PlanResponse": {
            "type": "object",
            "required": ["tenant_id", "plan"],
            "properties": {
                "tenant_id": { "type": "string", "description": "Tenant/store identifier" },
                "plan": { "type": "string", "enum": ["free", "pro"], "description": "Current sync plan" }
            }
        },
        "TerminalRegistrationRequest": {
            "type": "object",
            "required": ["terminal_id", "label"],
            "properties": {
                "terminal_id": { "type": "string", "description": "Unique terminal identifier", "example": "pos-terminal-1" },
                "label": { "type": "string", "description": "Human-readable label", "example": "Front Counter" },
                "tenant_id": { "type": "string", "description": "Optional tenant/store ID" }
            }
        },
        "TerminalRegistrationResponse": {
            "type": "object",
            "required": ["terminal_id", "secret"],
            "properties": {
                "terminal_id": { "type": "string", "description": "Registered terminal ID" },
                "secret": { "type": "string", "description": "Device secret — store securely; only shown once" }
            }
        },
        "SyncPushItem": {
            "type": "object",
            "required": ["id", "table_name", "action", "row_data"],
            "properties": {
                "id": { "type": "string", "description": "Queue item ID" },
                "table_name": { "type": "string", "description": "Target table", "example": "sales" },
                "action": { "type": "string", "enum": ["insert", "update", "delete"], "description": "CRUD action" },
                "row_data": { "type": "object", "description": "Serialized row payload" },
                "created_at": { "type": "string", "format": "date-time" }
            }
        },
        "SyncPushRequest": {
            "type": "array",
            "items": { "$ref": "#/components/schemas/SyncPushItem" },
            "description": "Array of offline queue items to push"
        },
        "SyncPullResponse": {
            "type": "object",
            "properties": {
                "items": { "type": "array", "items": { "$ref": "#/components/schemas/SyncPushItem" }, "description": "Pending items from other terminals" },
                "server_time": { "type": "string", "format": "date-time", "description": "Current server timestamp for the next pull's `since`" }
            }
        }
    })
}

// ── Paths builder ──────────────────────────────────────────────────────

fn build_paths() -> Value {
    json!({
        // ── Health ──────────────────────────────────────────────────
        "/health": {
            "get": {
                "tags": ["Health"],
                "summary": "Health check",
                "description": "Returns server status, version, DB connectivity, uptime, and sync queue depth. Public — no authentication required.",
                "operationId": "healthCheck",
                "responses": {
                    "200": { "description": "Server is healthy", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/HealthResponse" } } } },
                    "503": { "description": "Server is degraded (DB unreachable)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/health": {
            "get": {
                "tags": ["Health"],
                "summary": "Health check (API alias)",
                "description": "Alias for /health. Returns the same response.",
                "operationId": "healthCheckApi",
                "responses": {
                    "200": { "description": "Server is healthy", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/HealthResponse" } } } }
                }
            }
        },
        "/metrics": {
            "get": {
                "tags": ["Health"],
                "summary": "Prometheus metrics",
                "description": "Returns Prometheus text-format metrics including sync counters, health check metrics, and HTTP request histograms.",
                "operationId": "metricsEndpoint",
                "responses": {
                    "200": { "description": "Prometheus metrics in text/plain format", "content": { "text/plain": { "schema": { "type": "string" } } } }
                }
            }
        },
        "/api/v1/health": {
            "get": {
                "tags": ["Health"],
                "summary": "API health check",
                "description": "Lightweight health check returning status and version. Public — no authentication required. Simpler than /health (no DB ping).",
                "operationId": "apiHealthCheck",
                "responses": {
                    "200": { "description": "Server is healthy", "content": { "application/json": { "schema": { "type": "object", "required": ["status", "version"], "properties": { "status": { "type": "string", "example": "ok" },                 "version": { "type": "string", "example": env!("CARGO_PKG_VERSION") } } } } } }
                }
            }
        },

        // ── Auth (Tokens) ───────────────────────────────────────────
        "/api/v1/tokens": {
            "post": {
                "tags": ["Auth"],
                "summary": "Create a new API token",
                "description": "Generates a signed JWT for API authentication. Currently unprotected — will be gated behind an admin key in future.",
                "operationId": "createToken",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateTokenRequest" }, "example": { "label": "kitchen-display-1", "expiry_hours": 24, "tenant_id": "store-nyc" } } }
                },
                "responses": {
                    "200": { "description": "Token created successfully", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateTokenResponse" }, "example": { "token": { "token": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJraXRjaGVuLWRpc3BsYXktMSIsImV4cCI6MTc1MDAwMDAwMH0.abc123", "expires_at": "2026-08-13T00:00:00Z", "token_id": "550e8400-e29b-41d4-a716-446655440000" } } } } },
                    "400": { "description": "Invalid JSON body", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "415": { "description": "Unsupported content type", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "422": { "description": "Missing required field (label)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "500": { "description": "JWT encoding failed", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Terminals ──────────────────────────────────────────────
        "/api/v1/terminals": {
            "post": {
                "tags": ["Terminals"],
                "summary": "Register a new terminal",
                "description": "Registers a terminal for client-credential token minting (ADR sync-auth-hardening P3). Returns a device secret that must be stored securely — it is only returned once.",
                "operationId": "registerTerminal",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/TerminalRegistrationRequest" } } }
                },
                "responses": {
                    "201": { "description": "Terminal registered", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/TerminalRegistrationResponse" } } } },
                    "400": { "description": "Missing required field (terminal_id or label)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "409": { "description": "Terminal ID already registered", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Products ────────────────────────────────────────────────
        "/api/v1/products": {
            "get": {
                "tags": ["Products"],
                "summary": "List all products",
                "description": "Returns all products ordered by name, including category name and stock quantity. Requires JWT auth. Returns a flat array today; will adopt `PaginatedResponse` envelope when pagination support lands.",
                "operationId": "listProducts",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "$ref": "#/components/schemas/PaginationParams/limit" },
                    { "$ref": "#/components/schemas/PaginationParams/offset" },
                    { "$ref": "#/components/schemas/PaginationParams/sort" },
                    { "$ref": "#/components/schemas/PaginationParams/order" },
                    { "$ref": "#/components/schemas/PaginationParams/q" }
                ],
                "responses": {
                    "200": { "description": "List of products (may be empty)", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ProductDetail" } } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            },
            "post": {
                "tags": ["Products"],
                "summary": "Create a new product",
                "description": "Creates a product with optional category, barcode, and initial stock. SKU must be unique. Tenant ID is stamped from JWT claims.",
                "operationId": "createProduct",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateProductRequest" } } }
                },
                "responses": {
                    "201": { "description": "Product created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ProductDetail" } } } },
                    "400": { "description": "Validation error (empty SKU, empty name, negative price)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "409": { "description": "SKU already exists", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "500": { "description": "Internal server error", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/v1/products/{sku}": {
            "get": {
                "tags": ["Products"],
                "summary": "Get product by SKU",
                "description": "Returns full product detail including stock quantity. Returns JSON null when the SKU is not found (status 200).",
                "operationId": "getProduct",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "sku", "in": "path", "required": true, "schema": { "type": "string" }, "description": "Product SKU to look up" }
                ],
                "responses": {
                    "200": { "description": "Product detail, or null if not found", "content": { "application/json": { "schema": { "oneOf": [{ "$ref": "#/components/schemas/ProductDetail" }, { "type": "null" }] } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/v1/products/{sku}/stock": {
            "patch": {
                "tags": ["Products"],
                "summary": "Adjust stock quantity",
                "description": "Positive delta restocks, negative delta sells. The Store enforces non-negative stock with an atomic checked operation.",
                "operationId": "patchStock",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "sku", "in": "path", "required": true, "schema": { "type": "string" }, "description": "Product SKU to adjust" }
                ],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PatchStockRequest" } } }
                },
                "responses": {
                    "200": { "description": "Stock adjusted successfully", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PatchStockResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "404": { "description": "Product not found", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "422": { "description": "Adjustment would cause negative stock", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Plans (tenant sync plan — ADR sync-plan-gating) ────────
        "/api/v1/tenants/me/plan": {
            "get": {
                "tags": ["Plans"],
                "summary": "Get the caller's sync plan",
                "description": "Returns the tenant's cloud sync plan (free or pro) resolved from the JWT claims — a missing plan row reports free (fail closed). Unlike the sync router this endpoint is not plan-gated, so a free tenant can read its own plan to render the upgrade prompt.",
                "operationId": "getMyPlan",
                "security": [{ "bearerAuth": [] }],
                "responses": {
                    "200": { "description": "Effective plan for the authenticated tenant", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PlanResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/v1/tenants/{tenant_id}/plan": {
            "put": {
                "tags": ["Plans"],
                "summary": "Set a tenant's sync plan (admin)",
                "description": "Assigns free or pro to a tenant. Requires the X-Admin-Key header when OZ_ADMIN_KEY is configured; open in dev mode.",
                "operationId": "setTenantPlan",
                "parameters": [{ "name": "tenant_id", "in": "path", "required": true, "schema": { "type": "string" } }],
                "requestBody": {
                    "content": { "application/json": { "schema": { "type": "object", "properties": { "plan": { "type": "string", "enum": ["free", "pro"] } }, "required": ["plan"] } } }
                },
                "responses": {
                    "200": { "description": "Plan updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PlanResponse" } } } },
                    "400": { "description": "Unknown plan name", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Missing or invalid admin key", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Categories ──────────────────────────────────────────────
        "/api/v1/categories": {
            "get": {
                "tags": ["Categories"],
                "summary": "List all categories",
                "description": "Returns all product categories with ID, name, colour, and timestamps. Requires JWT auth. Returns a flat array today; will adopt `PaginatedResponse` envelope when pagination support lands.",
                "operationId": "listCategories",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "$ref": "#/components/schemas/PaginationParams/limit" },
                    { "$ref": "#/components/schemas/PaginationParams/offset" },
                    { "$ref": "#/components/schemas/PaginationParams/sort" },
                    { "$ref": "#/components/schemas/PaginationParams/order" }
                ],
                "responses": {
                    "200": { "description": "List of categories (may be empty)", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/CategoryDto" } } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Tax Rates ───────────────────────────────────────────────
        "/api/v1/tax-rates": {
            "post": {
                "tags": ["Tax Rates"],
                "summary": "Create a new tax rate",
                "description": "Creates a tax rate with basis-point precision (e.g., 1000 = 10%). Can be set as default and/or tax-inclusive.",
                "operationId": "createTaxRate",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateTaxRateRequest" } } }
                },
                "responses": {
                    "201": { "description": "Tax rate created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/TaxRateResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Users ───────────────────────────────────────────────────
        "/api/v1/users": {
            "post": {
                "tags": ["Users"],
                "summary": "Create a new user account",
                "description": "Creates a staff user with a PIN hash and role assignment. Requires seeded roles (role-staff, role-manager, role-owner).",
                "operationId": "createUser",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateUserRequest" } } }
                },
                "responses": {
                    "201": { "description": "User created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/UserResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Sales ───────────────────────────────────────────────────
        "/api/v1/sales": {
            "post": {
                "tags": ["Sales"],
                "summary": "Create a new sale",
                "description": "Creates a sale in 'pending' status with the given line items. Each line item specifies SKU, quantity, and unit price. At least one line item is required.",
                "operationId": "createSale",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateSaleRequest" }, "example": { "lines": [{ "sku": "COFFEE-001", "qty": 2, "unit_price": { "minor_units": 350, "currency": "USD" } }, { "sku": "MUFFIN-001", "qty": 1, "unit_price": { "minor_units": 425, "currency": "USD" } }] } } }
                },
                "responses": {
                    "201": { "description": "Sale created (status: pending)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SaleDetail" }, "example": { "id": "sale-abc123", "status": "pending", "lines": [{ "sku": "COFFEE-001", "qty": 2, "unit_price": { "minor_units": 350, "currency": "USD" } }], "total": { "minor_units": 1125, "currency": "USD" }, "created_at": "2026-08-12T10:30:00Z" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "422": { "description": "Empty lines array", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/v1/sales/{id}": {
            "get": {
                "tags": ["Sales"],
                "summary": "Get sale by ID",
                "description": "Returns full sale detail including line items and computed totals. Returns JSON null when the sale ID is not found (status 200).",
                "operationId": "getSale",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" }, "description": "Sale ID to retrieve" }
                ],
                "responses": {
                    "200": { "description": "Sale detail, or null if not found", "content": { "application/json": { "schema": { "oneOf": [{ "$ref": "#/components/schemas/SaleDetail" }, { "type": "null" }] } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/v1/sales/{id}/status": {
            "patch": {
                "tags": ["Sales"],
                "summary": "Update sale status",
                "description": "Transitions a sale through its lifecycle: pending→active (cart being processed), active→completed (payment received), active→voided (sale cancelled). Invalid transitions (e.g., pending→completed) return 422.",
                "operationId": "updateSaleStatus",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" }, "description": "Sale ID to update" }
                ],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/UpdateSaleStatusRequest" } } }
                },
                "responses": {
                    "200": { "description": "Status updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SaleStatusResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "404": { "description": "Sale not found", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "422": { "description": "Invalid status transition", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Sync ────────────────────────────────────────────────────
        "/api/sync/status": {
            "get": {
                "tags": ["Sync"],
                "summary": "Sync status",
                "description": "Returns the current state of the offline sync queue: pending count, conflict count, and total items. Scoped to the tenant in the JWT.\n\nRate limit headers returned when approaching per-tenant limits: `X-RateLimit-Remaining` (int), `X-RateLimit-Reset` (Unix timestamp), `Retry-After` (seconds).",
                "operationId": "syncStatus",
                "security": [{ "bearerAuth": [] }],
                "responses": {
                    "200": { "description": "Sync queue status", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SyncStatusResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/sync/push": {
            "post": {
                "tags": ["Sync"],
                "summary": "Push offline items to the server",
                "description": "Accepts a JSON array of offline queue items and stores them in the server's database. Each item is stamped with the tenant ID from the JWT for multi-tenant isolation.",
                "operationId": "syncPush",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "array",
                                "items": { "type": "object" },
                                "description": "Array of offline queue items to push"
                            }
                        }
                    }
                },
                "responses": {
                    "200": { "description": "Items accepted", "content": { "application/json": { "schema": { "type": "object", "properties": { "accepted": { "type": "integer", "format": "int64" } } } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "429": { "description": "Rate limited (per-tenant)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                },
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SyncPushRequest" } } }
                }
            }
        },
        "/api/sync/pull": {
            "post": {
                "tags": ["Sync"],
                "summary": "Pull pending items from the server",
                "description": "Returns items pushed by other terminals in the same tenant since the given timestamp. Each terminal polls this endpoint to stay in sync.",
                "operationId": "syncPull",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "since": { "type": ["string", "null"], "description": "ISO-8601 timestamp to filter items from" }
                                }
                            },
                            "example": { "since": null }
                        }
                    }
                },
                "responses": {
                    "200": { "description": "Items to sync (may be empty)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SyncPullResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "421": { "description": "Server migrated — use new URL", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Webhooks ────────────────────────────────────────────────
        "/api/webhooks/stripe": {
            "post": {
                "tags": ["Webhooks"],
                "summary": "Stripe webhook receiver",
                "description": "Receives Stripe webhook events. Payloads are verified using HMAC-SHA256 with the STRIPE_WEBHOOK_SECRET signing secret. Unauthenticated — verification is via the Stripe-Signature header. Subscription lifecycle events (customer.subscription.*, checkout.session.completed, invoice.paid) update the tenant's sync plan; payment events queue a finalize_sale action.",
                "operationId": "stripeWebhook",
                "requestBody": {
                    "content": { "application/json": { "schema": { "type": "object", "description": "Raw Stripe webhook event" } } }
                },
                "responses": {
                    "200": { "description": "Webhook processed successfully" },
                    "400": { "description": "Invalid signature or malformed event", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/webhooks/square": {
            "post": {
                "tags": ["Webhooks"],
                "summary": "Square webhook receiver",
                "description": "Receives Square webhook events. Payloads are verified using HMAC-SHA256 with the SQUARE_WEBHOOK_SIGNATURE_KEY. Unauthenticated — verification is via the x-square-hmacsha256-signature header.",
                "operationId": "squareWebhook",
                "requestBody": {
                    "content": { "application/json": { "schema": { "type": "object", "description": "Raw Square webhook event" } } }
                },
                "responses": {
                    "200": { "description": "Webhook processed successfully" },
                    "400": { "description": "Invalid signature or malformed event", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        }
    })
}

/// Handler: `GET /api/openapi.json` — returns the OpenAPI 3.1 specification.
pub async fn openapi_json_handler() -> Json<Value> {
    Json(openapi_spec())
}

/// Handler: `GET /api/docs` — returns the Swagger UI HTML page.
pub async fn swagger_ui_handler() -> Html<String> {
    swagger_ui_html()
}

/// Handler: `GET /api/docs/scalar` — returns the Scalar API Reference HTML page.
pub async fn scalar_ui_handler() -> Html<String> {
    scalar_html()
}

#[cfg(test)] #[path = "openapi_tests.rs"] mod tests;
