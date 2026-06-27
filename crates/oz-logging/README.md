# oz-logging

Structured logging facade for OZ-POS. Wraps the `tracing` ecosystem with a context-tagged record format (`[session_id][profile][task]`), a file + stdout writer, and a small CLI to inspect recent log output.

## Public API

- [`LoggingError`](src/error.rs) — `thiserror`-based error for the logging subsystem.

## Planned surface

- `init(config)` — install the `tracing-subscriber` with file + stdout writers.
- `LogContext` thread-local with scoped guards for automatic restore.
- A `log` CLI subcommand for tailing and filtering the local log file.
- Health-logger integration (memory + task success rate at a configurable interval).

## Status

Scaffold only. The file logger and context plumbing land in a follow-up.
