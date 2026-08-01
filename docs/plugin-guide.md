<!-- Audit stamp: 2026-07-31 · Buffy-Agent · status: SYNCED (PLG-10 parity rewrite) · verified against crates/oz-plugin (manager.rs, manifest.rs, loader.rs, package.rs) and crates/oz-lua (lib.rs, bridge.rs) · corrected: oz table surface (get_time/log/apply_discount/register_hook/on/off only), mandatory required_permissions + manifest validation (kebab-case name, strict SemVer, unknown-permission rejection), register_hook(string) signature, per-plugin env isolation, HAL traits table (no NfcReader), oz-cli commands (no run-script/validate-plugins), sandbox limits (100k instr / 10 MB) -->

# OZ-POS Plugin System

Plugins extend OZ-POS with custom business logic, hardware drivers,
and integrations — all without modifying the core codebase.

## Plugin Manifest (`plugin.toml`)

Every plugin is a directory containing a `plugin.toml` manifest. The manifest
is **validated at load time** (PLG-08): invalid manifests fail loudly with an
actionable error and the plugin is not loaded.

```toml
[plugin]
name = "my-custom-discount"   # kebab-case: lowercase letters/digits/hyphens, 1-64 chars
version = "1.0.0"             # strict SemVer (e.g. "1.2.3-beta.1")
description = "A custom discount rule for Tuesday afternoons"
author = "My Company"
license = "MIT"

[capabilities]
# Scripts that the plugin provides (paths must stay inside the plugin dir)
scripts = ["discount.lua", "validation.lua"]

# Hooks the plugin listens to (informational; lowercase/digits/dot/underscore/hyphen)
hooks = ["sale.before_complete"]

[permissions]
# REQUIRED: at least one permission must be declared, and every permission
# must be recognised — unknown permissions reject the plugin.
required_permissions = ["cart:read", "cart:write", "system:time", "log:write"]
```

### Available permissions

| Permission | Grants the `oz` binding |
|------------|--------------------------|
| `cart:read` | `oz.register_hook`, `oz.on`, `oz.off` |
| `cart:write` | `oz.apply_discount` |
| `tax:read` | *(reserved — tax-rate read access)* |
| `inventory:read` | *(reserved — stock read access)* |
| `inventory:write` | *(reserved — stock adjustment)* |
| `reporting:read` | *(reserved — reporting/analytics access)* |
| `system:time` | `oz.get_time` |
| `log:write` | `oz.log` |

Bindings whose permission is not granted simply do not exist in the plugin's
`oz` table, so an unapproved call fails fast in the sandbox.

## Plugin Directory Structure

```
plugins/
  my-custom-discount/
    plugin.toml
    discount.lua
    validation.lua
  my-receipt-printer/
    plugin.toml
    printer.lua
```

## Discovery

Plugins are loaded from the `plugins/` directory at startup:

1. OZ-POS scans `plugins/` (relative to the app data directory)
2. Each subdirectory with a `plugin.toml` is loaded; manifest schema violations
   fail loudly instead of silently skipping
3. Each plugin's Lua scripts load into **its own isolated environment** — plugin
   globals never leak between plugins or into the shared namespace
4. Scripts can register hooks by calling `oz.register_hook(name, function_name)`
5. Plugin IDs must be unique; duplicate IDs are rejected

## Sandbox

Lua scripts run in a hardened sandbox:

- **No filesystem or network access**: `os.execute`, `os.remove`, `os.rename`,
  and `os.exit` are `nil`; read-only `os.date`/`os.time`/`os.clock` remain
- **Instruction limit**: scripts abort after 100 000 Lua instructions
  (prevents infinite loops)
- **Memory limit**: the Lua VM is capped at 10 MB (prevents memory exhaustion)
- **Isolated environments**: each plugin loads into its own `_ENV`, with `_G`
  pointed at that environment — a plugin writing `_G.foo = ...` cannot affect
  any other plugin

## `oz` Global Table

Only the following bindings are implemented. Each is capability-gated by the
plugin's declared permissions (see above).

| Function | Permission | Description |
|----------|------------|-------------|
| `oz.log(level, message)` | `log:write` | Log a message (`level`: "info", "warn", "error", "debug") |
| `oz.get_time()` | `system:time` | Current time table: `wday`, `hour`, `min`, `sec`, `month`, `day`, `year` |
| `oz.apply_discount(target, percent)` | `cart:write` | Queue a discount: `"cart"` or `"line:<SKU>"`; `percent` must be 0–100 |
| `oz.register_hook(event, function_name)` | `cart:read` | Register a hook by **function name** (string), resolved in this plugin's environment |
| `oz.on(event, callback)` | `cart:read` | Register an inline callback function |
| `oz.off(event)` | `cart:read` | Unsubscribe this plugin's callbacks for an event (a plugin can only ever remove its own) |

### Legacy top-level hooks

In addition to `oz.register_hook`, the runtime still recognises these
**top-level functions** defined in a plugin script (each resolved in the
plugin's own environment):

| Lua function | Signature | Called when |
|---|---|---|
| `apply_discount` | `(lines_json) → {percent, label} \| nil` | Before sale creation |
| `calc_line_tax` | `(sku, qty, unit_price_minor, currency) → {rate_bps, is_inclusive} \| nil` | During tax computation |
| `validate_order` | `(lines_json, total_minor, currency) → string[]` | Before completion |

## Example: Custom Discount

```lua
-- plugins/tuesday-discount/discount.lua
function apply_tuesday_discount(sale)
  local now = oz.get_time()
  if now.wday == 3 then  -- Tuesday
    oz.log("info", "Tuesday 10% discount applied")
    oz.apply_discount("cart", 10)  -- 10% off entire cart
  end
end

-- register_hook takes the function NAME as a string
oz.register_hook("sale.before_complete", "apply_tuesday_discount")
```

Requires `required_permissions = ["cart:read", "cart:write", "system:time", "log:write"]`
in `plugin.toml` (see `plugins/example-discount/` for a complete example).

## HAL Driver API Surface

Third-party hardware drivers implement the traits defined in `crates/oz-hal/`.

### Available Driver Traits (v1.0)

| Trait | Crate | Description |
|-------|-------|-------------|
| `BarcodeScanner` | `oz-hal` | Connect, poll for scans, cancel pending reads |
| `ReceiptPrinter` | `oz-hal` | Print receipts, barcodes, QR codes, cash drawer kick |
| `CashDrawer` | `oz-hal` | Open drawer, detect drawer state |
| `CustomerDisplay` | `oz-hal` | Show/hide messages, update totals |

### Implementing a Custom Driver

See `crates/oz-hal/examples/custom_barcode_scanner.rs` for a complete,
tested example of implementing the `BarcodeScanner` trait for custom hardware.

Key requirements:
1. Implement the trait methods (`connect`, `poll`, `cancel`, `device_info`)
2. Return `oz_hal::HalError` for all error paths

> **Note:** Native driver loading from `plugin.toml` is not yet wired into the
> Lua runtime. Drivers are implemented in Rust against the `oz-hal` traits; the
> `capabilities.drivers` manifest field is currently informational.

## Security

- Lua scripts run in a sandbox with no filesystem or network access
- CPU time and memory are limited (100 000 instructions / 10 MB)
- Each plugin loads into its own isolated environment; hooks are owner-tagged
- Plugins can only ever unsubscribe their own hooks/callbacks
- Manifests are validated: kebab-case plugin IDs, strict SemVer, recognised
  permissions only, unique IDs, and script paths confined to the plugin
  directory (no `..`, absolute paths, or symlink escapes)
- `.ozpkg` archives are parsed with path-traversal and zip-bomb protections

## Creating a Plugin

1. Create a directory in `plugins/`
2. Write your `plugin.toml` (including at least one `required_permissions`)
3. Write your Lua scripts
4. Restart OZ-POS to load the plugin
5. Check the logs for any load errors

## Testing Plugins

The `oz-plugin` crate includes an integration test that loads the real
`plugins/example-discount` plugin end-to-end:

```bash
cargo test -p oz-plugin --lib
```

## Troubleshooting

| Symptom | Likely Cause |
|---------|-------------|
| Plugin not loaded, "invalid manifest" | Missing/invalid `plugin.toml`; unknown permission; bad name or version format |
| Plugin not loaded, "unsafe script path" | A declared script escapes the plugin directory |
| Plugin not loaded, "duplicate plugin id" | Two plugins declare the same `plugin.name` |
| Lua errors on startup | Syntax error in script — check logs |
| `attempt to call a nil value` on `oz.*` | The plugin lacks the permission for that binding |
| Hook not firing | `oz.register_hook` needs `cart:read`; check the event name and function name |
