# oz-lua

Embedded Lua scripting runtime for OZ-POS — lets merchants customize business
rules, promotions, and order validation at runtime without recompiling the core.

## Status

**Stable.** The runtime is built on `rlua` with a sandboxed environment that
strips dangerous globals (`os`, `io`, `loadfile`, etc.) and exposes three hooks:

| Hook | Signature | Purpose |
|------|-----------|---------|
| `apply_discount` | `(lines_table) → {percent, label} \| nil` | Return a % discount or nil |
| `calc_line_tax` | `(sku, qty, unit_price, currency) → {rate_bps, is_inclusive} \| nil` | Override tax rate per line |
| `validate_order` | `(lines_table, total_minor, currency) → string[]` | Return validation errors |

Scripts live in `scripts/` and are loaded at startup via `load_dir()`.

## Example

```lua
function apply_discount(lines)
    local total = 0
    for i = 1, #lines do
        total = total + lines[i].qty * lines[i].unit_price_minor
    end
    if total > 5000 then
        return { percent = 5, label = "Volume discount" }
    end
    return nil
end
```

## Sandboxing

- `os`, `io`, `loadfile`, `dofile`, `require`, `package`, `debug`, `rawget`,
  `rawset`, `collectgarbage`, `module`, `load` are **removed**.
- Safe libraries preserved: `math`, `string`, `table`, `pairs`, `ipairs`,
  `tonumber`, `tostring`, `type`, `pcall`, `xpcall`, `error`.

## Tests

```
cargo test --package oz-lua
> 18 passed, 0 failed
```

> last audited 2026-06-29 by docs-auditor
