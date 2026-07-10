# ADR #7: Data Scope Guard & Query Enforcement

**Status:** In Progress (2026-07-10)
**Date:** 2026-07-10
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** security, session, scope, database, enforcement, ADR #4 follow-up

---

## Context

ADR #4 established the session token infrastructure: opaque UUID v4 tokens created by `create_session`, stored in an in-memory `HashMap<String, SessionContext>`, and resolved by `resolve_session()`. The frontend's `WorkspaceContext` manages the token lifecycle — creating tokens on workspace selection, destroying them on logout/store-switch.

However, ADR #4 explicitly deferred three enforcement concerns to this ADR:

1. **`session_context()` extractor**: A standardized way for Tauri commands to resolve scope from a token.
2. **Domain command migration**: All commands (`list_orders`, `get_products`, `adjust_stock`, etc.) must accept `SessionContext` instead of raw `store_id`/`user_id` parameters.
3. **`clippy` lint rule**: Reject `store_id: String` in command parameter signatures at compile time.

This ADR designs the enforcement layer that transforms the "soft" session token pattern from ADR #4 into a "hard" compile-time guarantee that no command can access data outside its resolved scope.

---

## Decision

### 1. The `resolve_scope()` Helper

Instead of a middleware or extractor (Tauri v2 has neither), we provide a convenience method on `AppState`:

```rust
impl AppState {
    /// Resolve a session token and open the store-scoped database.
    /// Returns the SessionContext and a locked Connection for the store's SQLite database.
    pub fn resolve_scope(
        &self,
        token: &str,
    ) -> Result<(SessionContext, MutexGuard<'_, Connection>), AppError> {
        let session = self.resolve_session(token)?;
        let conn = self.db_manager.open_store(&session.store_id)?;
        let db = conn.lock()?;
        Ok((session, db))
    }
}
```

This is the **canonical entry point** for all domain commands. It:
1. Validates the session token → `InvalidSession` if invalid
2. Opens the correct per-store database file → error if store doesn't exist
3. Locks the connection → `PoisonError` → `Internal` error
4. Returns both the `SessionContext` (for audit logging, permission checks) and the `Connection` (for queries)

### 2. Domain Command Migration Pattern

Every domain command follows this template:

```rust
#[command]
pub async fn list_products_scoped(
    state: State<'_, AppState>,
    session_token: String,
) -> Result<Vec<ProductDto>, AppError> {
    let (_session, db) = state.resolve_scope(&session_token)?;
    run_list_products(&db)  // existing business logic, unchanged
}
```

**Migration strategy for existing commands:**

| Command | Current params | Migrated params | Status |
|---|---|---|---|
| `list_products` | `()` | `session_token: String` | ✅ `list_products_scoped` added, old preserved |
| `adjust_stock` | `args: AdjustStockArgs` | `session_token: String, args: AdjustStockArgs` | ✅ `adjust_stock_scoped` added, API wrapper `adjustStockScoped` |
| `lookup_by_barcode` | `barcode: String` | `session_token: String, barcode: String` | ⏳ |
| `lookup_product_by_sku` | `sku: String` | `session_token: String, sku: String` | ⏳ |
| `create_product` | `args: CreateProductArgs` (has user_id) | `session_token: String, args` (remove user_id) | ⏳ |
| `update_product` | `args: UpdateProductArgs` (has user_id) | `session_token: String, args` (remove user_id) | ⏳ |
| `delete_product` | `args: DeleteProductArgs` (has user_id) | `session_token: String, args` (remove user_id) | ⏳ |
| `list_orders` | (needs investigation) | `session_token: String, ...` | ⏳ |
| *(all other domain commands)* | various | `session_token: String, ...` | ⏳ |

### 3. Compile-Time Enforcement (Clippy Lint)

A custom Clippy lint rule will reject any Tauri `#[command]` function that accepts `store_id: String` as a direct parameter. This is tracked separately as a clippy plugin task and is the **last step** in the migration — after all commands are migrated, the lint prevents regressions.

**Lint rule (pseudocode):**
```
For every #[command] fn:
  If any parameter is named `store_id` with type `String`:
    Emit: "Use session_token + resolve_scope() instead of raw store_id"
```

This lint runs in CI but is **not** enforced locally during development (to avoid blocking feature work). It only fires on PRs after the migration is complete.

---

## Implementation Plan

### Phase 1: Infrastructure (ADR #4) ✅
- [x] `SessionContext` struct
- [x] `session_store: Arc<RwLock<HashMap<String, SessionContext>>>`
- [x] `resolve_session()` on `AppState`
- [x] `create_session` / `destroy_session` Tauri commands
- [x] Frontend token lifecycle in `WorkspaceContext`

### Phase 2: `resolve_scope()` Helper ✅
- [x] `resolve_scope()` on desktop `AppState`
- [x] `resolve_scope()` on tablet `AppState`
- [x] `list_products_scoped` simplified to use `resolve_scope()`

### Phase 3: Domain Command Migration ⏳
- [ ] `adjust_stock_scoped` — migrate stock adjustment
- [ ] `lookup_by_barcode_scoped` — migrate barcode lookup
- [ ] `lookup_product_by_sku_scoped` — migrate SKU lookup
- [ ] `create_product_scoped` — remove `user_id` from args, use session token
- [ ] `update_product_scoped` — remove `user_id` from args
- [ ] `delete_product_scoped` — remove `user_id` from args
- [ ] `list_orders_scoped` — migrate order listing
- [ ] *(remaining domain commands)*

### Phase 4: Enforcement ⏳
- [ ] Custom Clippy lint rule: reject `store_id: String` in command params
- [ ] CI integration: lint runs on PRs
- [ ] Backward-compatible deprecation period (old commands preserved)

---

## Consequences

### Positive
- Every domain command gets store isolation with a single `.resolve_scope()` call.
- The session token is validated on every command — stale tokens fail immediately.
- Business logic (`run_list_products`, etc.) is unchanged — only the command wrapper changes.
- Compile-time lint prevents accidental `store_id` parameters after migration.

### Negative
- Every command must be migrated individually (mechanical, but many files).
- The `resolve_scope()` helper blocks the async runtime thread during DB lock (acceptable for SQLite reads/writes measured in microseconds).

---

## Related

- ADR #4 — Store-First Tenancy & Workspace Type/Instance Architecture
- `apps/desktop-client/src/state.rs` — `AppState::resolve_session()`, `resolve_scope()`
- `apps/desktop-client/src/commands/products.rs` — Reference implementation (`list_products_scoped`)
- `ui/src/api/products.ts` — Frontend API wrapper (`listProductsScoped`)
- `ui/src/contexts/WorkspaceContext.tsx` — Session token lifecycle
