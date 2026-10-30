# Rust CPU benchmark report

Date: 2026-10-30. Host: AMD Ryzen 9 6900HX, 8 cores / 16 threads, Chimera
Linux. Rust 1.99.0, release profile (`lto = "thin"`, `codegen-units = 1`).
Reproduce with `cd rust && cargo bench`, `cargo test --release`, and the
commands below.

## Micro-benchmarks (`cargo bench`)

Representative run:

| Operation | Result |
|---|---:|
| Delta-table build (parallel) | 41.0 ms |
| Full score | 20.9 us/op, 47,879 ops/s |
| Single-swap delta | 15.5 us/op, 64,412 ops/s |
| Delta-scored quench | 223.5 ms |
| Full-score quench (`quench_generic`) | 1,026.7 ms |
| Anneal, 10k iterations | 166.1 ms, 60,213 iters/s |

The delta-scored quench is **4.6x** faster than the full-score quench on the
same layout and move set. Delta scoring is most valuable for the two-pair
cross terms used by quench; single swaps still touch many trigrams, so the
raw single-swap delta is only modestly cheaper than a full score.

