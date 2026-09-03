//! Shared OpenAPI 3.1 document for the `oz-api` surface.
//!
//! Single source of truth for the contract served by BOTH the cloud
//! server (`apps/cloud-server`, which merges its own sync/webhook/docs
//! paths on top) and the desktop local API
//! (`apps/desktop-client/src/local_api.rs`, which serves exactly this
//! base document). Every operation carries `x-oz-scope`:
//!
//! - `"both"` — served by the cloud server AND the desktop local API
//!   (everything in this module);
//! - `"cloud"` — cloud-only (added by the cloud server's builder).
//!
//! The document is generated programmatically (no utoipa dependency —
//! spec 0047 records the migration as deliberately deferred). Drift is
//! policed by `apps/cloud-server/src/openapi_tests.rs`: the merged spec
//! must equal the route set registered in the router sources, in BOTH
//! directions.

use serde_json::{Value, json};

/// Scope value: operation served by cloud server and desktop local API.
pub const SCOPE_BOTH: &str = "both";
/// Scope value: cloud-server-only operation (sync, webhooks, docs UI).
pub const SCOPE_CLOUD: &str = "cloud";

/// The OpenAPI 3.1 document for the shared (`x-oz-scope: "both"`)
/// surface: exactly what `router()` serves, plus the self-documenting
/// `/api/openapi.json` path that both embedders register.
pub fn base_spec() -> Value {
    let mut spec = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "OZ-POS API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "REST API for the OZ-POS point-of-sale system.\n\n## Authentication\nMost endpoints require a JWT bearer token from `POST /api/v1/tokens`. Pass it as `Authorization: Bearer <token>`.\n\n## Endpoint scope\nEvery operation carries `x-oz-scope`: `both` means the endpoint is served by the cloud server AND the desktop app's loopback local API (Settings → Local API); `cloud` means cloud-server-only (sync, webhooks, docs UI). This document describes the `both` surface; the cloud server serves the merged superset at its own `/api/openapi.json`.\n\n## Versioning\nThe API is versioned by URL path prefix (`/api/v1/`). Breaking changes will ship under a new version prefix (`/api/v2/`) — the old version remains available for at least 6 months after the new one lands.\n\n## Pagination\nList endpoints accept `?limit` (default 50, max 200) and `?offset` (default 0) query parameters and return a `PaginatedResponse` envelope with `data`, `total`, `limit`, and `offset` fields.\n\n## Errors\nAll error responses share a common envelope: `{ \"error\": { \"code\": \"MACHINE_READABLE\", \"message\": \"Human description\", \"details\": [...] } }`. The `code` field is stable across versions — use it for programmatic error handling, not the message string.\n\n## Changelog\n- **Read tiers (0.0.34, spec 0047):** terminal client-credential tokens now bind the `terminal` preset — reads are gated by `permissions` claim keys (403 `insufficient_scope` when missing). Legacy tokens without the claim keep full read. The `OZ_TERMINAL_READ_TIER=full` escape hatch restores legacy terminal reads and is **deprecated** (removal after one release cycle).",
            "contact": { "name": "OZ-POS" }
        },
        "externalDocs": {
            "description": "OZ-POS documentation",
            "url": "https://github.com/oz-pos/oz-pos"
        },
        "tags": [
            { "name": "Health", "description": "Server health and monitoring endpoints" },
            { "name": "Auth", "description": "Token generation and authentication. **Read tiers (spec 0047):** a token may carry an optional `permissions` claim (list of permission-registry keys). Reads are then gated per route: a missing key returns `403 insufficient_scope`. A token without the claim is grandfathered as full-read. Presets (`terminal`, `dashboard`, `audit`) are mint-time sugar — terminal client-credential tokens automatically bind the `terminal` preset; admin-key mints accept `read_preset`/`read_permissions`. The `OZ_TERMINAL_READ_TIER=full` escape hatch preserves legacy terminal reads (deprecated, one release window)." },
            { "name": "Products", "description": "Product CRUD and stock management" },
            { "name": "Images", "description": "Content-addressed product/menu-item image store (spec 0046b)" },
            { "name": "Categories", "description": "Product category listing" },
            { "name": "Tax Rates", "description": "Tax rate configuration" },
            { "name": "Exchange Rates", "description": "Currency exchange rate management (global reference data)" },
            { "name": "Users", "description": "User account management" },
            { "name": "Sales", "description": "Sale creation, retrieval, and status transitions" },
            { "name": "Plans", "description": "Tenant cloud sync plans (ADR sync-plan-gating)" },
            { "name": "Terminals", "description": "Terminal registration for client-credential authentication" },
            { "name": "Settings", "description": "Per-tenant cloud settings provisioning (SMTP, report schedule, store name). Gated by the admin key (X-Admin-Key), not by JWT — see the operation descriptions." },
            { "name": "Docs", "description": "Self-describing API documentation endpoints (OpenAPI 3.1 document)" },
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
            "schemas": build_base_schemas()
        },
        "paths": build_base_paths()
    });
    annotate_scope(&mut spec["paths"], SCOPE_BOTH);
    spec
}

/// The document the desktop local API serves: [`base_spec`] with
/// loopback server info and local-facing wording injected.
pub fn local_spec(port: u16) -> Value {
    let mut spec = base_spec();
    spec["info"]["title"] = json!("OZ-POS Local Terminal API");
    spec["info"]["description"] = json!(
        "REST API served by the OZ-POS desktop app on this machine only \
         (127.0.0.1). Enable it in Settings → Local API. Tokens are minted \
         in that panel; every operation here is `x-oz-scope: \"both\"` — \
         cloud-only endpoints (sync, webhooks, docs UI) are absent. This \
         server reads and writes the PRIMARY STORE's database — the same \
         data the register UI shows; note `POST /api/v1/users` therefore \
         does not create register login accounts (identity lives outside \
         this server). See docs/guides/EXTENDING.md in the source tree \
         for scripting recipes.\n\n\
         ## Authentication\nJWT bearer from the Settings panel: \
         `Authorization: Bearer <token>`. Master-data writes additionally \
         require `X-Admin-Key` (the per-install secret — see the guide §2.2)."
    );
    spec["servers"] = json!([
        { "url": format!("http://127.0.0.1:{port}"), "description": "This terminal (loopback only)" }
    ]);
    spec
}

/// Insert `x-oz-scope: <scope>` into every operation object under
/// `paths` (mutating in place). Path-item keys that are not HTTP
/// operations (`parameters`, `summary`, …) are skipped.
pub fn annotate_scope(paths: &mut Value, scope: &str) {
    let Some(items) = paths.as_object_mut() else {
        return;
    };
    for item in items.values_mut() {
        let Some(op_map) = item.as_object_mut() else {
            continue;
        };
        for (key, operation) in op_map.iter_mut() {
            if !is_operation_key(key) {
                continue;
            }
            if let Some(op) = operation.as_object_mut() {
                op.insert("x-oz-scope".to_string(), json!(scope));
            }
        }
    }
}

/// True for OpenAPI path-item operation keys (HTTP verbs).
pub fn is_operation_key(key: &str) -> bool {
    matches!(
        key,
        "get" | "put" | "post" | "delete" | "options" | "head" | "patch" | "trace"
    )
}

// ── Base schemas (shared surface) ────────────────────────────────────

fn build_base_schemas() -> Value {
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
        "CreateTokenRequest": {
            "type": "object",
            "required": ["label"],
            "properties": {
                "label": { "type": "string", "description": "Human-readable label for the token", "example": "kitchen-display-1" },
                "expiry_hours": { "type": "integer", "format": "int64", "description": "Expiry in hours (default: 24)", "example": 24 },
                "tenant_id": { "type": "string", "description": "Optional tenant/store ID for multi-tenant isolation (admin-key path only — the client-credentials path takes the tenant from the terminal's registration, never the body)" },
                "client_id": { "type": "string", "description": "Registered terminal ID — client-credentials mint path (ADR sync-auth-hardening P3); paired with client_secret, no admin key needed" },
                "client_secret": { "type": "string", "description": "Device secret from terminal registration (verified against the stored SHA-256 hash)" },
                "read_preset": { "type": "string", "enum": ["terminal", "dashboard", "audit"], "description": "Read-tier preset (spec 0047) — admin-key mint path only; terminal client-credentials always bind `terminal` server-side" },
                "read_permissions": { "type": "array", "items": { "type": "string" }, "description": "Explicit permission-registry keys for the token's read tier — admin-key path only; overrides read_preset when both are present" }
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
                "updated_at": { "type": "string", "format": "date-time" },
                "image_hash": { "type": ["string", "null"], "description": "Slot-1 primary image content hash (spec 0046b)" },
                "images": { "type": "array", "items": { "$ref": "#/components/schemas/ProductImage" }, "description": "Content-addressed image assignments (slots 1..5)" }
            }
        },
        "ProductImage": {
            "type": "object",
            "properties": {
                "slot": { "type": "integer", "description": "1 = primary, 2..5 = alternatives" },
                "hash": { "type": "string", "description": "16-hex content hash" },
                "position": { "type": "integer", "description": "Display order of alternatives (0-based)" }
            }
        },
        "PutImageResponse": {
            "type": "object",
            "required": ["hash16"],
            "properties": {
                "hash16": { "type": "string", "description": "16-hex content hash of the stored image" }
            }
        },
        "BatchImageResult": {
            "type": "object",
            "properties": {
                "hash": { "type": ["string", "null"], "description": "Content hash when accepted; null when rejected" },
                "status": { "type": "string", "enum": ["stored", "duplicate", "rejected"], "description": "Per-hash outcome" }
            }
        },
        "BatchPutResponse": {
            "type": "object",
            "properties": {
                "results": { "type": "array", "items": { "$ref": "#/components/schemas/BatchImageResult" }, "description": "Per-hash outcomes, in the same order as the request frames" }
            }
        },
        "MissingHashesResponse": {
            "type": "object",
            "properties": {
                "missing_hashes": { "type": "array", "items": { "type": "string" }, "description": "Candidate hashes the tenant has no active image_refs row for" }
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
        "CreateExchangeRateRequest": {
            "type": "object",
            "required": ["from_currency", "to_currency", "rate_millionths"],
            "properties": {
                "from_currency": { "type": "string", "description": "ISO-4217 alpha-3 source currency code", "example": "USD" },
                "to_currency": { "type": "string", "description": "ISO-4217 alpha-3 target currency code", "example": "IDR" },
                "rate_millionths": { "type": "integer", "format": "int64", "description": "Fixed-point rate at 6-decimal scale (16000000 = 16.0), strictly positive", "example": 16000000 },
                "source": { "type": "string", "description": "Provenance label; defaults to 'manual'", "example": "manual" },
                "effective_date": { "type": "string", "description": "YYYY-MM-DD; defaults to today (UTC)", "example": "2026-08-31" }
            }
        },
        "ExchangeRateResponse": {
            "type": "object",
            "required": ["id", "from_currency", "to_currency", "rate_millionths", "source", "effective_date", "created_at"],
            "properties": {
                "id": { "type": "string", "description": "Row id (UUID v7)" },
                "from_currency": { "type": "string", "example": "USD" },
                "to_currency": { "type": "string", "example": "IDR" },
                "rate_millionths": { "type": "integer", "format": "int64", "example": 16000000 },
                "source": { "type": "string", "example": "manual" },
                "effective_date": { "type": "string", "example": "2026-08-31" },
                "created_at": { "type": "string", "description": "RFC-3339 creation timestamp" }
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
            "required": ["terminal_id"],
            "properties": {
                "terminal_id": { "type": "string", "description": "Unique terminal identifier", "example": "pos-terminal-1" },
                "label": { "type": "string", "description": "Human-readable label (optional; stored as empty string when omitted)", "example": "Front Counter" },
                "tenant_id": { "type": "string", "description": "Optional tenant/store ID" }
            }
        },
        "TerminalRegistrationResponse": {
            "type": "object",
            "required": ["terminal_id", "device_secret"],
            "properties": {
                "terminal_id": { "type": "string", "description": "Registered terminal ID (also the `client_id` for token minting)" },
                "device_secret": { "type": "string", "description": "Device secret — store securely; shown exactly once, never retrievable. Only its SHA-256 hash is persisted. Re-registering the same terminal_id ROTATES this secret (old credentials stop working immediately)." }
            }
        },
        "SmtpConfig": {
            "type": "object",
            "required": ["host", "port", "from", "use_tls"],
            "properties": {
                "host": { "type": "string", "description": "SMTP server hostname" },
                "port": { "type": "integer", "description": "SMTP port (25, 465, 587, …)", "example": 587 },
                "username": { "type": ["string", "null"], "description": "Optional authenticated-relay username" },
                "password": { "type": ["string", "null"], "description": "Relay password — encrypted at rest server-side; returned DECRYPTED by GET (admin round-trip, API-2 tradeoff)" },
                "from": { "type": "string", "description": "From-address for outgoing emails" },
                "use_tls": { "type": "boolean", "description": "STARTTLS (true) or plaintext (false); port 465 uses implicit TLS" }
            }
        },
        "ReportScheduleConfig": {
            "type": "object",
            "required": ["enabled", "cadence", "report_types", "recipients", "send_at_time", "timezone", "lookback_days"],
            "properties": {
                "enabled": { "type": "boolean", "description": "Whether scheduled delivery is on" },
                "cadence": { "type": "string", "description": "\"daily\", \"weekly\", \"monthly\", or a cron expression", "example": "daily" },
                "report_types": { "type": "array", "items": { "type": "string" }, "description": "Report types to include", "example": ["daily_revenue", "top_products"] },
                "recipients": { "type": "array", "items": { "type": "string" }, "description": "Recipient email addresses" },
                "send_at_time": { "type": "string", "description": "Time of day to send", "example": "08:00" },
                "timezone": { "type": "string", "description": "IANA timezone for scheduling", "example": "Asia/Jakarta" },
                "lookback_days": { "type": "integer", "description": "Date-range window in days", "example": 7 }
            }
        },
        "SettingsView": {
            "type": "object",
            "required": ["tenant"],
            "description": "Effective per-tenant settings (scoped key first, bare-key fallback). Returned by GET and after a successful PUT.",
            "properties": {
                "tenant": { "type": "string", "description": "Tenant these settings belong to" },
                "store_name": { "type": ["string", "null"], "description": "Effective store display name" },
                "smtp_config": { "type": ["object", "null"], "description": "Effective SMTP config (password decrypted), or null", "additionalProperties": true },
                "report_schedule": { "type": ["object", "null"], "description": "Effective report schedule, or null", "additionalProperties": true },
                "last_report_sent_at": { "type": ["string", "null"], "description": "Last-sent dedup timestamp, or null" }
            }
        },
        "PutSettingsRequest": {
            "type": "object",
            "description": "Field-level upsert: an ABSENT field is left untouched, an explicit `null` deletes the tenant's scoped override (bare key applies again), a value writes the scoped key.",
            "properties": {
                "tenant": { "type": "string", "default": "default", "description": "Tenant to write; `[a-zA-Z0-9_-]`, max 64 chars" },
                "store_name": { "type": ["string", "null"], "description": "Store display name override (null = delete)" },
                "smtp_config": { "type": ["object", "null"], "description": "SMTP config override validated against SmtpConfig (null = delete); password encrypted at rest on write", "additionalProperties": true },
                "report_schedule": { "type": ["object", "null"], "description": "Report schedule override validated against ReportScheduleConfig (null = delete)", "additionalProperties": true }
            }
        }
    })
}

// ── Base paths (the `x-oz-scope: "both"` surface) ──────────────

fn build_base_paths() -> Value {
    json!({
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
                "description": "Generates a signed JWT (HS256, default expiry 24 h, no revocation list — keep `expiry_hours` short). Two mint paths: (1) **admin-key path** — requires the `X-Admin-Key` header when the server has an admin key configured (open in dev mode); optionally narrows reads via `read_preset`/`read_permissions`. (2) **terminal client-credentials path** — `client_id` + `client_secret` from a registered terminal (ADR sync-auth-hardening P3); no admin key, tenant taken from the registration, reads bound to the `terminal` preset server-side (spec 0047).",
                "operationId": "createToken",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateTokenRequest" }, "example": { "label": "kitchen-display-1", "expiry_hours": 24, "tenant_id": "store-nyc" } } }
                },
                "responses": {
                    "200": { "description": "Token created successfully", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateTokenResponse" }, "example": { "token": { "token": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJraXRjaGVuLWRpc3BsYXktMSIsImV4cCI6MTc1MDAwMDAwMH0.abc123", "expires_at": "2026-08-13T00:00:00Z", "token_id": "550e8400-e29b-41d4-a716-446655440000" } } } } },
                    "400": { "description": "Invalid JSON body", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Admin key configured but missing/mismatched (`invalid_admin_key`), or terminal client credentials rejected (`invalid_credentials`)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "415": { "description": "Unsupported content type", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "422": { "description": "Missing required field (label), or unknown `read_preset` (`unknown_preset`) / `read_permissions` key (`unknown_permission`)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "500": { "description": "JWT encoding failed", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Terminals ──────────────────────────────────────────────
        "/api/v1/terminals": {
            "post": {
                "tags": ["Terminals"],
                "summary": "Register a new terminal",
                "description": "Registers a terminal for client-credential token minting (ADR sync-auth-hardening P3). Returns a `device_secret` that must be stored securely — it is shown exactly once (only its SHA-256 hash is persisted). Re-registering an existing `terminal_id` ROTATES the secret (upsert; old credentials stop working immediately — there is no 409). Gated by the server's admin key (X-Admin-Key header); open in dev mode.",
                "operationId": "registerTerminal",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/TerminalRegistrationRequest" } } }
                },
                "responses": {
                    "200": { "description": "Terminal registered (or re-registered — secret rotated)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/TerminalRegistrationResponse" } } } },
                    "400": { "description": "Blank terminal_id", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Admin key configured but missing/mismatched (`invalid_admin_key`)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "500": { "description": "Registration persistence failed", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
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
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `products:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
                }
            },
            "post": {
                "tags": ["Products"],
                "summary": "Create a new product",
                "description": "Creates a product with optional category, barcode, and initial stock. SKU must be unique. Tenant ID is stamped from JWT claims. **Operator-tier (D1):** requires the `X-Admin-Key` header when the server has an admin key configured, and rejects terminal-scoped tokens.",
                "operationId": "createProduct",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateProductRequest" } } }
                },
                "responses": {
                    "201": { "description": "Product created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ProductDetail" } } } },
                    "400": { "description": "Validation error (empty SKU, empty name, negative price)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Missing or invalid JWT, or missing/invalid `X-Admin-Key` when configured", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Terminal-scoped token cannot write master data", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } },
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
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `products:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
                }
            }
        },
        "/api/v1/products/{sku}/stock": {
            "patch": {
                "tags": ["Products"],
                "summary": "Adjust stock quantity",
                "description": "Positive delta restocks, negative delta sells. The Store enforces non-negative stock with an atomic checked operation. **Operator-tier (D1):** requires the `X-Admin-Key` header when the server has an admin key configured, and rejects terminal-scoped tokens.",
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
                    "401": { "description": "Missing or invalid JWT, or missing/invalid `X-Admin-Key` when configured", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Terminal-scoped token cannot write master data", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } },
                    "404": { "description": "Product not found", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "422": { "description": "Adjustment would cause negative stock", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Images (product/menu-item content store — spec 0046b) ──
        "/api/v1/images": {
            "put": {
                "tags": ["Images"],
                "summary": "Upload a single product image",
                "description": "Body is the raw WebP bytes (max 32 KB). The server re-verifies magic bytes + size and recomputes sha-256 before storing atomically on the volume. Returns the 16-hex content hash — the filename, ETag, and cache key in one.",
                "operationId": "putImage",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "hash", "in": "query", "required": false, "schema": { "type": "string" }, "description": "Optional client-computed hash; if present it must match the server's (409 otherwise)." }
                ],
                "requestBody": {
                    "required": true,
                    "content": { "application/octet-stream": { "schema": { "type": "string", "format": "binary" } } }
                },
                "responses": {
                    "201": { "description": "Image stored (or already present as a content-addressed duplicate)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PutImageResponse" } } } },
                    "400": { "description": "Not a valid WebP or exceeds 32 KB", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "409": { "description": "Client-supplied hash does not match the computed hash", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            },
            "post": {
                "tags": ["Images"],
                "summary": "Upload a batch of product images",
                "description": "Body is length-prefixed binary frames (big-endian u32 length + bytes) for up to 16 images / 512 KB. The server re-verifies each file and answers per-hash `stored|duplicate|rejected` in the same order.",
                "operationId": "putImageBatch",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/octet-stream": { "schema": { "type": "string", "format": "binary" } } }
                },
                "responses": {
                    "201": { "description": "Batch processed (per-hash outcomes)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/BatchPutResponse" } } } },
                    "400": { "description": "Malformed frames or all images rejected", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "413": { "description": "Batch exceeds limits (16 images / 512 KB)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/v1/images:pack": {
            "get": {
                "tags": ["Images"],
                "summary": "Cold-start pack of images",
                "description": "Returns up to 64 files / 2 MB as length-prefixed frames for the given comma-separated hashes. Missing or unreferenced hashes are silently skipped. Used by fresh tablet provisioning instead of thousands of per-hash GETs.",
                "operationId": "getImagePack",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "hashes", "in": "query", "required": true, "schema": { "type": "string" }, "description": "Comma-separated list of content hashes" }
                ],
                "responses": {
                    "200": { "description": "Length-prefixed image frames (may be empty)", "content": { "application/octet-stream": { "schema": { "type": "string", "format": "binary" } } } },
                    "400": { "description": "No valid hashes or more than 64 requested", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `products:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
                }
            }
        },
        "/api/v1/images:missing": {
            "get": {
                "tags": ["Images"],
                "summary": "Server-side missing-hash nudge",
                "description": "Given comma-separated candidate hashes, returns the subset the tenant has no active `image_refs` row for. The desktop push scheduler calls this before a batch upload so it pushes exactly what the cloud lacks first (spec 0046b §3.6).",
                "operationId": "getImageMissing",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "hashes", "in": "query", "required": true, "schema": { "type": "string" }, "description": "Comma-separated list of candidate content hashes" }
                ],
                "responses": {
                    "200": { "description": "The missing subset", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MissingHashesResponse" } } } },
                    "400": { "description": "No valid hashes", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `products:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
                }
            }
        },
        "/api/v1/images/{hash16}": {
            "get": {
                "tags": ["Images"],
                "summary": "Fetch an image by content hash",
                "description": "Returns the immutable WebP bytes for a content-addressed hash the tenant references. `Cache-Control: max-age=31536000, immutable` and `ETag: \"<hash>\"` — every cache layer between the tablet and the volume can treat it as a static asset. Unknown or un-hashed files return 404.",
                "operationId": "getImage",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "hash16", "in": "path", "required": true, "schema": { "type": "string", "minLength": 16, "maxLength": 16 }, "description": "16-hex content hash" }
                ],
                "responses": {
                    "200": { "description": "Immutable WebP bytes", "content": { "image/webp": { "schema": { "type": "string", "format": "binary" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `products:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } },
                    "404": { "description": "Invalid hash grammar, unknown hash, or tenant has no active reference", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Plans (tenant sync plan — ADR sync-plan-gating) ────────
        "/api/v1/tenants/me/plan": {            "get": {
                "tags": ["Plans"],
                "summary": "Get the caller's sync plan",
                "description": "Returns the tenant's cloud sync plan (free or pro) resolved from the JWT claims — a missing plan row reports free (fail closed). Unlike the sync router this endpoint is not plan-gated, so a free tenant can read its own plan to render the upgrade prompt.",
                "operationId": "getMyPlan",
                "security": [{ "bearerAuth": [] }],
                "responses": {
                    "200": { "description": "Effective plan for the authenticated tenant", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PlanResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `plan:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
                }
            }
        },
        "/api/v1/tenants/{tenant_id}/plan": {
            "put": {
                "tags": ["Plans"],
                "summary": "Set a tenant's sync plan (admin)",
                "description": "Assigns free or pro to a tenant. Requires the X-Admin-Key header when the server has an admin key configured; open in dev mode.",
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

        // ── Settings ────────────────────────────────────────────────
        "/api/v1/settings": {
            "get": {
                "tags": ["Settings"],
                "summary": "Read a tenant's effective cloud settings",
                "description": "Returns the tenant's effective settings exactly as the cloud report loop resolves them: scoped key `{base}:{tenant}` first, bare-key fallback. Includes `store_name`, `smtp_config` (password DECRYPTED for lossless admin round-trips — API-2 tradeoff, safe only while the admin-key gate + mandatory production key hold), `report_schedule`, and `last_report_sent_at`. Gated by the server's admin key (X-Admin-Key header); open in dev mode. No JWT.",
                "operationId": "getSettings",
                "parameters": [
                    { "name": "tenant", "in": "query", "required": false, "schema": { "type": "string", "default": "default" }, "description": "Tenant to read; `[a-zA-Z0-9_-]`, max 64 chars" }
                ],
                "responses": {
                    "200": { "description": "Effective settings for the tenant", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SettingsView" } } } },
                    "400": { "description": "Invalid tenant id charset (`invalid_tenant`)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Admin key configured but missing/mismatched (`invalid_admin_key`)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "500": { "description": "Settings read failed (`settings_read_failed`)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            },
            "put": {
                "tags": ["Settings"],
                "summary": "Write a tenant's scoped settings",
                "description": "Field-level upsert of the tenant's scoped keys (`store.name:{tenant}`, `smtp_config:{tenant}`, `report_schedule:{tenant}`). A field that is ABSENT is left untouched; an explicit `null` deletes the scoped override so the bare key applies again. Every provided field is validated and canonicalized BEFORE any write, so a bad request never leaves a half-applied config. SMTP passwords are encrypted at rest on write. Same admin-key gate as GET.",
                "operationId": "putSettings",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PutSettingsRequest" } } }
                },
                "responses": {
                    "200": { "description": "Effective settings after the write", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SettingsView" } } } },
                    "400": { "description": "Validation failure (`invalid_tenant`, `invalid_store_name`, `invalid_smtp_config`, `invalid_report_schedule`)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Admin key configured but missing/mismatched (`invalid_admin_key`)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "500": { "description": "Write or re-read failed (`settings_write_failed` / `settings_read_failed`)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
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
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `categories:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
                }
            }
        },

        // ── Tax Rates ───────────────────────────────────────────────
        "/api/v1/tax-rates": {
            "post": {
                "tags": ["Tax Rates"],
                "summary": "Create a new tax rate",
                "description": "Creates a tax rate with basis-point precision (e.g., 1000 = 10%). Can be set as default and/or tax-inclusive. **Operator-tier (D1):** requires the `X-Admin-Key` header when the server has an admin key configured, and rejects terminal-scoped tokens.",
                "operationId": "createTaxRate",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateTaxRateRequest" } } }
                },
                "responses": {
                    "201": { "description": "Tax rate created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/TaxRateResponse" } } } },
                    "401": { "description": "Missing or invalid JWT, or missing/invalid `X-Admin-Key` when configured", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Terminal-scoped token cannot write master data", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
                }
            }
        },

        // ── Exchange Rates ──────────────────────────────────────────
        "/api/v1/exchange-rates": {
            "get": {
                "tags": ["Exchange Rates"],
                "summary": "List exchange rate history",
                "description": "Full rate history ordered pair-major with newest effective date first within each pair (CUR-04). Rates are global reference data.",
                "operationId": "listExchangeRates",
                "security": [{ "bearerAuth": [] }],
                "responses": {
                    "200": { "description": "Rate history (may be empty)", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ExchangeRateResponse" } } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `reference:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
                }
            },
            "post": {
                "tags": ["Exchange Rates"],
                "summary": "Create an exchange rate",
                "description": "Creates a rate with 6-decimal fixed-point precision. Rejects non-positive rates, identical pairs, non-ISO codes, and malformed effective dates (CUR-05). Duplicate (pair, date) returns 409. **Operator-tier (D1):** requires the `X-Admin-Key` header when the server has an admin key configured, and rejects terminal-scoped tokens.",
                "operationId": "createExchangeRate",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateExchangeRateRequest" } } }
                },
                "responses": {
                    "201": { "description": "Rate created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ExchangeRateResponse" } } } },
                    "400": { "description": "Validation error", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Missing or invalid JWT, or missing/invalid `X-Admin-Key` when configured", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Terminal-scoped token cannot write master data", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } },
                    "409": { "description": "Rate already exists for this pair and effective date", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/v1/exchange-rates/latest": {
            "get": {
                "tags": ["Exchange Rates"],
                "summary": "List the current rate for every pair",
                "description": "One row per (from_currency, to_currency) pair — the newest effective date (CUR-11 bounded listing).",
                "operationId": "listLatestExchangeRates",
                "security": [{ "bearerAuth": [] }],
                "responses": {
                    "200": { "description": "Current rates (may be empty)", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ExchangeRateResponse" } } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `reference:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
                }
            }
        },
        "/api/v1/exchange-rates/latest/{from}/{to}": {
            "get": {
                "tags": ["Exchange Rates"],
                "summary": "Get the newest rate for one pair",
                "description": "Path codes are case-insensitive; 404 when the pair has no rates.",
                "operationId": "getLatestExchangeRate",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "from", "in": "path", "required": true, "schema": { "type": "string" }, "description": "Source currency code" },
                    { "name": "to", "in": "path", "required": true, "schema": { "type": "string" }, "description": "Target currency code" }
                ],
                "responses": {
                    "200": { "description": "Newest rate for the pair", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ExchangeRateResponse" } } } },
                    "400": { "description": "Invalid ISO-4217 code", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `reference:read` read-tier permission", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } },
                    "404": { "description": "No rate for this pair", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },
        "/api/v1/exchange-rates/{id}": {
            "delete": {
                "tags": ["Exchange Rates"],
                "summary": "Delete an exchange rate",
                "description": "Removes a rate row by id. **Operator-tier (D1):** requires the `X-Admin-Key` header when the server has an admin key configured, and rejects terminal-scoped tokens.",
                "operationId": "deleteExchangeRate",
                "security": [{ "bearerAuth": [] }],
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" }, "description": "Rate row id" }
                ],
                "responses": {
                    "204": { "description": "Rate deleted" },
                    "401": { "description": "Missing or invalid JWT, or missing/invalid `X-Admin-Key` when configured", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Terminal-scoped token cannot write master data", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } },
                    "404": { "description": "Rate not found", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
                }
            }
        },

        // ── Users ───────────────────────────────────────────────────
        "/api/v1/users": {
            "post": {
                "tags": ["Users"],
                "summary": "Create a new user account",
                "description": "Creates a staff user with a PIN hash and role assignment. Requires seeded roles (role-staff, role-manager, role-owner). Requires an admin-minted token: a token scoped to a registered terminal is rejected with 403, since user management is an admin-tier operation.",
                "operationId": "createUser",
                "security": [{ "bearerAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreateUserRequest" } } }
                },
                "responses": {
                    "201": { "description": "User created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/UserResponse" } } } },
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token is scoped to a registered terminal and may not manage users", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } }
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
                    "401": { "description": "Missing or invalid JWT", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "403": { "description": "Token lacks the `sales:view` read-tier permission (PII-flagged route)", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" }, "example": { "error": "insufficient_scope" } } } }
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

        // ── Docs ────────────────────────────────────────────────────
        "/api/openapi.json": {
            "get": {
                "tags": ["Docs"],
                "summary": "OpenAPI 3.1 specification",
                "description": "This document. Public — no authentication required.",
                "operationId": "openapiJson",
                "responses": {
                    "200": { "description": "OpenAPI 3.1 document", "content": { "application/json": { "schema": { "type": "object" } } } }
                }
            }
        },
    })
}

#[cfg(test)]
#[path = "spec_tests.rs"]
mod tests;
