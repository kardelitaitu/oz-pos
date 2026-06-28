# oz-logging

Structured logging facade wrapping the `tracing` ecosystem.

## Public API

| Function | Format | Output |
|----------|--------|--------|
| `init()` | Human-readable text | stdout |
| `init_json()` | Newline-delimited JSON | stdout |
| `init_with_file(dir, prefix, days)` | Human-readable text | stdout + rolling file |
| `init_json_with_file(dir, prefix, days)` | JSON | stdout + rolling file |

All four read `RUST_LOG` (default `info`). File appender rotates hourly; files older than `retention_days` are cleaned up.

```rust
oz_logging::init();                                      // dev
oz_logging::init_json_with_file("logs", "oz-pos", 30);   // production
```

### Platform modules

| Module | Platform | Output |
|--------|----------|--------|
| `syslog` | Linux | Syslog |
| `eventlog` | Windows | Event Log |

## Conventions

- `init()` should be called once, early in `main`/`run`, before any `tracing` macro.
- `#![warn(missing_docs)]`.

> last audited 28-06-26 by docs-auditor
