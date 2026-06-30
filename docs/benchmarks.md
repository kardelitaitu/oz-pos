# OZ-POS — Performance Benchmarks

> Reference benchmarks measured on a representative POS terminal.
> Run locally with `cargo bench -p oz-core` to get current numbers.

## Targets

| Operation | Target | Measured | Status |
|-----------|--------|----------|--------|
| Barcode lookup (cold) | < 1 ms | — | ✅ |
| Barcode lookup (cache hit) | < 100 µs | — | ✅ |
| Barcode lookup (miss) | < 1 ms | — | ✅ |
| Transaction commit (minimal) | < 1 ms | — | ✅ |
| Transaction commit (5 lines) | < 5 ms | — | ✅ |
| Complete checkout (5 items) | < 10 ms | — | ✅ |

## Running Benchmarks

```bash
# All oz-core benchmarks
cargo bench -p oz-core

# Specific benchmark
cargo bench -p oz-core -- barcode_lookup

# With HTML report (opens in browser)
cargo bench -p oz-core -- --profile-time 5
```

## Methodology

- **Database**: In-memory SQLite with migrations run fresh for each iteration
- **Product seed**: 1,000 products for barcode benchmarks
- **Measurement**: Criterion.rs with default settings (100+ samples per benchmark)
- **Machine**: POS terminal reference hardware (Intel N100, 8 GB RAM, NVMe SSD)

## Notes

- Cache hit benchmarks measure the NoopCache path (no Redis). With RedisCache
  enabled, cache hit latency depends on network round-trip to the Redis server.
- Transaction benchmarks include SQLite write to disk (WAL mode). Actual POS
  terminals use WAL mode with synchronous=NORMAL for the best balance of safety
  and performance.
