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

## Available Lua API

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
