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
            "description": "REST API for the OZ-POS point-of-sale cloud sync server. Provides product management, sales processing, category listing, tax rate configuration, user management, token-based authentication, sync push/pull endpoints, and webhook receivers.",
            "contact": { "name": "OZ-POS" }
        },
        "servers": [
            { "url": "http://localhost:3099", "description": "Local development server" }
        ],
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
            { "name": "Webhooks", "description": "Third-party payment provider webhook receivers" }
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
                "version": { "type": "string", "description": "Server version", "example": "0.0.9" },
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
                "status": { "type": "string", "description": "New status: 'active', 'completed', or 'voided'", "enum": ["active", "completed", "voided"] }
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

        // ── Auth (Tokens) ───────────────────────────────────────────
        "/api/v1/tokens": {
            "post": {
                "tags": ["Auth"],
                "summary": "Create a new API token",
                "description": "Generates a signed JWT for API authentication. Currently unprotected — will be gated behind an admin key in future.",
                "operationId": "createToken",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateTokenRequest" } } }
                },
                "responses": {
                    "200": { "description": "Token created successfully", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateTokenResponse" } } } },
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
                "description": "Returns all products ordered by name, including category name and stock quantity. Requires JWT auth.",
                "operationId": "listProducts",
                "security": [{ "bearerAuth": [] }],
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
                    "200": { "description": "Product detail, or null if not found" },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            },
            "parameters": [
                { "name": "sku", "in": "path", "required": true, "schema": { "type": "string" } }
            ]
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
                "description": "Returns all product categories with ID, name, colour, and timestamps. Requires JWT auth.",
                "operationId": "listCategories",
                "security": [{ "bearerAuth": [] }],
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
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateSaleRequest" } } }
                },
                "responses": {
                    "201": { "description": "Sale created (status: pending)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SaleDetail" } } } },
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
                "description": "Returns the current state of the offline sync queue: pending count, conflict count, and total items. Scoped to the tenant in the JWT.",
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> axum::Router {
        use axum::routing::get;
        axum::Router::new()
            .route("/api/openapi.json", get(openapi_json_handler))
            .route("/api/docs", get(swagger_ui_handler))
            .route("/api/docs/scalar", get(scalar_ui_handler))
    }

    #[tokio::test]
    async fn openapi_json_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/openapi.json")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn openapi_json_has_required_fields() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/openapi.json")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["openapi"], "3.1.0");
        assert!(json["info"]["title"].is_string());
        assert!(json["info"]["version"].is_string());
        assert!(json["paths"].is_object());
        assert!(json["components"]["schemas"].is_object());
        assert!(json["components"]["securitySchemes"]["bearerAuth"].is_object());
    }

    #[tokio::test]
    async fn openapi_json_documents_all_tag_groups() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/openapi.json")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        let tags: Vec<&str> = json["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(tags.contains(&"Health"));
        assert!(tags.contains(&"Auth"));
        assert!(tags.contains(&"Products"));
        assert!(tags.contains(&"Categories"));
        assert!(tags.contains(&"Tax Rates"));
        assert!(tags.contains(&"Users"));
        assert!(tags.contains(&"Sales"));
        assert!(tags.contains(&"Sync"));
        assert!(tags.contains(&"Plans"));
        assert!(tags.contains(&"Terminals"));
        assert!(tags.contains(&"Webhooks"));
    }

    #[tokio::test]
    async fn openapi_json_documents_all_paths() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/openapi.json")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        let paths = json["paths"].as_object().unwrap();
        assert!(paths.contains_key("/health"), "missing /health");
        assert!(paths.contains_key("/api/health"), "missing /api/health");
        assert!(paths.contains_key("/metrics"), "missing /metrics");
        assert!(paths.contains_key("/api/v1/tokens"), "missing tokens");
        assert!(paths.contains_key("/api/v1/products"), "missing products");
        assert!(
            paths.contains_key("/api/v1/products/{sku}"),
            "missing product by SKU"
        );
        assert!(
            paths.contains_key("/api/v1/products/{sku}/stock"),
            "missing stock adjustment"
        );
        assert!(
            paths.contains_key("/api/v1/tenants/me/plan"),
            "missing self plan read"
        );
        assert!(
            paths.contains_key("/api/v1/tenants/{tenant_id}/plan"),
            "missing plan set"
        );
        assert!(
            paths.contains_key("/api/v1/categories"),
            "missing categories"
        );
        assert!(paths.contains_key("/api/v1/tax-rates"), "missing tax rates");
        assert!(paths.contains_key("/api/v1/users"), "missing users");
        assert!(paths.contains_key("/api/v1/sales"), "missing sales");
        assert!(
            paths.contains_key("/api/v1/sales/{id}"),
            "missing sale by ID"
        );
        assert!(
            paths.contains_key("/api/v1/sales/{id}/status"),
            "missing sale status update"
        );
        assert!(
            paths.contains_key("/api/v1/terminals"),
            "missing terminal registration"
        );
        assert!(
            paths.contains_key("/api/sync/status"),
            "missing sync status"
        );
        assert!(paths.contains_key("/api/sync/push"), "missing sync push");
        assert!(paths.contains_key("/api/sync/pull"), "missing sync pull");
        assert!(
            paths.contains_key("/api/webhooks/stripe"),
            "missing stripe webhook"
        );
        assert!(
            paths.contains_key("/api/webhooks/square"),
            "missing square webhook"
        );
    }

    #[tokio::test]
    async fn swagger_ui_returns_html() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/docs")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("swagger-ui"));
        assert!(html.contains("/api/openapi.json"));
    }

    #[tokio::test]
    async fn scalar_ui_returns_html() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/docs/scalar")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("api-reference"));
        assert!(html.contains("/api/openapi.json"));
    }

    #[test]
    fn openapi_spec_is_valid_json() {
        let spec = openapi_spec();
        // Verify it serializes without panicking
        let _json_str = serde_json::to_string_pretty(&spec).unwrap();
    }

    #[test]
    fn security_scheme_has_bearer_auth() {
        let spec = openapi_spec();
        let security = &spec["components"]["securitySchemes"]["bearerAuth"];
        assert_eq!(security["type"], "http");
        assert_eq!(security["scheme"], "bearer");
        assert_eq!(security["bearerFormat"], "JWT");
    }

    #[test]
    fn all_protected_routes_have_security() {
        let spec = openapi_spec();
        let paths = spec["paths"].as_object().unwrap();

        // Routes that should have bearerAuth security
        let protected_routes = [
            ("/api/v1/products", "get"),
            ("/api/v1/products", "post"),
            ("/api/v1/products/{sku}", "get"),
            ("/api/v1/products/{sku}/stock", "patch"),
            ("/api/v1/categories", "get"),
            ("/api/v1/tax-rates", "post"),
            ("/api/v1/users", "post"),
            ("/api/v1/sales", "post"),
            ("/api/v1/sales/{id}", "get"),
            ("/api/v1/sales/{id}/status", "patch"),
            ("/api/sync/status", "get"),
            ("/api/sync/push", "post"),
            ("/api/sync/pull", "post"),
        ];

        for (path, method) in &protected_routes {
            let operation = &paths[*path][*method];
            let security = operation
                .get("security")
                .unwrap_or_else(|| panic!("{method} {path} must have security defined"));
            let has_bearer = security
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s.as_object().unwrap().contains_key("bearerAuth"));
            assert!(has_bearer, "{method} {path} must have bearerAuth security");
        }
    }

    #[test]
    fn health_endpoints_have_no_security() {
        let spec = openapi_spec();
        let paths = spec["paths"].as_object().unwrap();

        for path in &["/health", "/api/health", "/metrics"] {
            let operation = &paths[*path]["get"];
            assert!(
                operation.get("security").is_none()
                    || operation
                        .get("security")
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .is_empty(),
                "{path} should not require security"
            );
        }
    }
}
