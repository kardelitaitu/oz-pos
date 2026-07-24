<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: STALE (2 findings — aspirational plugin API not fully implemented) · F1 (line 101): lists NfcReader as an available v1.0 HAL driver trait; crates/oz-hal/src/traits/ has BarcodeScanner, ReceiptPrinter, CashDrawer, CustomerDisplay but NO NfcReader trait · F2 (lines 179-182): documents `cargo run -p oz-cli -- run-script` and `-- validate-plugins` subcommands; oz-cli Command enum (cli.rs) has Migrate/InitDb/Product/Backup/... and NO run-script or validate-plugins · verified accurate: crates/oz-hal/examples/custom_barcode_scanner.rs exists; oz.register_hook + oz.apply_discount present in oz-lua; sandbox (no fs/network) matches oz-lua -->

# OZ-POS Plugin System

Plugins extend OZ-POS with custom business logic, hardware drivers,
and integrations — all without modifying the core codebase.

## Plugin Manifest (`plugin.toml`)

Every plugin is a directory containing a `plugin.toml` manifest:

```toml
[plugin]
name = "my-custom-discount"
version = "1.0.0"
description = "A custom discount rule for Tuesday afternoons"
author = "My Company"
license = "MIT"

[capabilities]
# Scripts that the plugin provides
scripts = ["discount.lua", "validation.lua"]

# Hardware drivers the plugin registers (optional)
# drivers = ["my-barcode-scanner"]

# Hooks the plugin listens to
# hooks = ["sale.before_complete", "product.after_lookup"]

[permissions]
# What the plugin can access
allow_network = false
allow_filesystem = false
allow_http = false
```

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
2. Each subdirectory with a `plugin.toml` is loaded
3. Lua scripts are loaded into a sandboxed VM
4. Scripts can register hooks by calling `oz.register_hook(name, function)`

## API Versioning

The OZ-POS Plugin API follows **semantic versioning** independent of the
main application version. The current stable API version is **v1.0**.

### Version Guarantees

| Guarantee | Description |
|-----------|-------------|
| **Backward compatibility** | Plugins written for API v1.0 will work on all future v1.x releases |
| **Deprecation notice** | Deprecated APIs are marked with `@deprecated` for at least one minor version before removal |
| **Migration path** | Breaking changes go through a major version bump (v2.0) with documented migration guides |
| **Feature detection** | Plugins can check `oz.api_version` at runtime to conditionally use newer APIs |

### Checking API Version at Runtime

```lua
local api_ver = oz.api_version()
if api_ver.major >= 2 then
  oz.log("info", "Using v2+ API features")
end
```

### Deprecation Policy

1. APIs marked for deprecation log a warning on first use
2. Deprecated APIs remain functional for at least one minor version
3. Removal happens only in a major version bump
4. Migration guides are published with each major version

## HAL Driver API Surface

Third-party hardware drivers implement the traits defined in `crates/oz-hal/`.
Each trait is versioned independently and follows the same backward compatibility
guarantees as the Lua API.

### Available Driver Traits (v1.0)

| Trait | Crate | Description |
|-------|-------|-------------|
| `BarcodeScanner` | `oz-hal` | Connect, poll for scans, cancel pending reads |
| `ReceiptPrinter` | `oz-hal` | Print receipts, barcodes, QR codes, cash drawer kick |
| `CashDrawer` | `oz-hal` | Open drawer, detect drawer state |
| `CustomerDisplay` | `oz-hal` | Show/hide messages, update totals |
| `NfcReader` | `oz-hal` | Read NFC tags/cards, emulate tags |

### Implementing a Custom Driver

See `crates/oz-hal/examples/custom_barcode_scanner.rs` for a complete,
tested example of implementing the `BarcodeScanner` trait for custom hardware.

Key requirements:
1. Implement the trait methods (`connect`, `poll`, `cancel`, `device_info`)
2. Return `oz_hal::HalError` for all error paths
3. Register the driver via `plugin.toml`:

```toml
[drivers]
barcode_scanner = { type = "custom", path = "my_scanner.lua" }
receipt_printer = { type = "escpos", vendor_id = "0x04b8", product_id = "0x0202" }
```

## API Changelog

### v1.0 (current)

- `oz.log(level, message)`
- `oz.get_setting(key)`
- `oz.get_product(sku)`
- `oz.get_cart()`
- `oz.apply_discount(line_or_cart, percent)`
- `oz.calc_line_tax(line)`
- `oz.get_time()`
- `oz.register_hook(name, function)`
- `oz.api_version()`
- HAL traits: BarcodeScanner, ReceiptPrinter, CashDrawer, CustomerDisplay, NfcReader

### `oz` Global Table

| Function | Description |
|----------|-------------|
| `oz.log(level, message)` | Log a message (level: "info", "warn", "error") |
| `oz.get_setting(key)` | Read a store setting |
| `oz.get_product(sku)` | Get product details (returns table) |
| `oz.get_cart()` | Get the current cart contents |
| `oz.apply_discount(line_or_cart, percent)` | Apply percentage discount |
| `oz.calc_line_tax(line)` | Calculate tax for a line item |

### Example: Custom Discount

```lua
-- plugins/tuesday-discount/discount.lua
function on_before_complete(sale)
  local now = oz.get_time()
  if now.wday == 3 then  -- Tuesday
    oz.log("info", "Tuesday discount applied")
    oz.apply_discount("cart", 10)  -- 10% off entire cart
  end
end

oz.register_hook("sale.before_complete", on_before_complete)
```

## Security

- Lua scripts run in a sandbox with no filesystem or network access
- CPU time is limited (configurable timeout)
- Memory is limited (configurable limit)
- All plugin scripts are scanned for suspicious patterns at load time

## Creating a Plugin

1. Create a directory in `plugins/`
2. Write your `plugin.toml`
3. Write your Lua scripts
4. Restart OZ-POS to load the plugin
5. Check the logs for any load errors

## Testing Plugins

```bash
# Run a plugin script directly via the CLI
cargo run -p oz-cli -- run-script plugins/my-plugin/discount.lua

# Validate all plugin manifests
cargo run -p oz-cli -- validate-plugins
```

## Troubleshooting

| Symptom | Likely Cause |
|---------|-------------|
| Plugin not loaded | Missing or invalid `plugin.toml` |
| Lua errors on startup | Syntax error in script — check logs |
| Hook not firing | Plugin not permitted to use that hook |
| Permission denied | Plugin requested `allow_network` but not granted |
