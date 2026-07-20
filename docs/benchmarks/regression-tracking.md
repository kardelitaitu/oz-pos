# Benchmark Regression Tracking

> Historical tracking of all OZ-POS benchmarks. Each entry records the
> baseline and delta since the previous measurement.

## How to update

```bash
# Run all benchmarks
cargo bench -p oz-core

# Compare against stored baseline
cargo install critcmp
critcmp baseline current

# If performance has improved intentionally, update the baseline:
cp target/criterion/baseline.json docs/benchmarks/baseline.json
```

## Tracking Table

| Date | Suite | Benchmark | Baseline (µs) | Current (µs) | Delta % | Trend |
|------|-------|-----------|---------------|-------------|---------|-------|
| 2026-07-20 | transaction_commit | `create_sale_minimal` | 20.4 | — | — | — |
| 2026-07-20 | transaction_commit | `create_sale_with_5_lines` | 46.5 | — | — | — |
| 2026-07-20 | transaction_commit | `complete_checkout_5_items` | 47.4 | — | — | — |
| 2026-07-20 | barcode_lookup | `barcode_lookup_1000_products` | < 1.0 | — | — | — |
| 2026-07-20 | barcode_lookup | `barcode_lookup_cache_hit` | < 0.1 | — | — | — |
| 2026-07-20 | barcode_lookup | `barcode_lookup_miss` | < 0.5 | — | — | — |
| 2026-07-20 | money_bench | `money_checked_add` | < 0.01 | — | — | — |
| 2026-07-20 | money_bench | `money_checked_sub` | < 0.01 | — | — | — |
| 2026-07-20 | money_bench | `money_checked_mul` | < 0.015 | — | — | — |
| 2026-07-20 | money_bench | `money_checked_div` | < 0.015 | — | — | — |
| 2026-07-20 | money_bench | `money_serde_roundtrip` | < 1.0 | — | — | — |
| 2026-07-20 | cart_bench | `cart_add_line` | < 0.5 | — | — | — |
| 2026-07-20 | cart_bench | `cart_total` | < 0.1 | — | — | — |

## Trend Legend

| Symbol | Meaning |
|--------|---------|
| ↑ | Regression (> 10% slower) — investigate |
| ↓ | Improvement (> 10% faster) — update baseline |
| → | Stable (within ±10%) — no action needed |
| — | Initial measurement — no comparison available |

## CI Integration

The `benchmarks` job in `.github/workflows/ci.yml` runs on push to `main`:
- Executes `cargo bench -p oz-core`
- Compares against stored baseline via `critcmp`
- Fails if any benchmark regresses > 10%
- Updates baseline on intentional improvements (requires manual approval)

## Storage

- Baselines: `docs/benchmarks/baseline.json` (git-tracked)
- HTML reports: GitHub Actions artifacts (90-day retention)
- Historical graphs: Criterion HTML report in `target/criterion/report/`
