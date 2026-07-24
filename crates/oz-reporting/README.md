<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: STALE (1 finding — contradicts actual code) · F1: README says "Scaffold only — ReportingError defined. Reports land once cart, sale, payment, inventory tables stabilize" — but the crate is now IMPLEMENTED: src/ contains daily_summary.rs, menu_engineering.rs, metrics.rs (real report engines) plus error.rs/lib.rs; ReportingError still present. Update to describe the actual engines · verified: error.rs + ReportingError exist; crate is no longer scaffold-only (mirrors oz-lua/oz-security/oz-payment which the doc itself still calls "scaffold") -->

# oz-reporting

Analytics and CSV export engine for OZ-POS (planned).

## Status

Scaffold only — `ReportingError` defined. Reports land once cart, sale, payment, and inventory tables stabilize.

> last audited 28-06-26 by docs-auditor
