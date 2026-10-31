# Rust rewrite completion summary

Date: 2026-10-30. Status: complete. The Rust binary in `rust/` is the primary
implementation; the Python tree is frozen as the parity oracle.

## Phases

| Phase | Result | Evidence |
|---|---|---|
| Simple port | All seven runtime commands ported, numerically equivalent to Python | `PARITY_REPORT.md` |
| CPU optimization | Delta scoring, faster quench/ILS, parallel seeds, benchmark harness | `BENCHMARKS.md` |
| GPU | Not shipped; `--device gpu` falls back to CPU with a warning | `GPU_FEASIBILITY.md` |
| Tuning | Plateau reached; defaults set to `iters=200000 runs=28 ils=8 polish=0` | `TUNING_REPORT.md` |
| Diversity | Near-equal layouts recorded for review | `DIVERSITY_REPORT.md` |

## Final gates

```text
cargo fmt --check                                   OK
cargo clippy --all-targets -- -D warnings           OK
cargo test --release                                OK
rust/target/release/shirolyzer check-baseline       baseline OK: audit 166/166, 9 references
rust/target/release/shirolyzer analyze ...           matches Python analysis JSON/table
rust/target/release/shirolyzer compare               matches Python
rust/target/release/shirolyzer audit ...             rulings: 166/166
```

## Intentional differences and open items

- Exploratory `compute` runs use a different RNG family from Python, so random
  restarts are quality-comparable rather than decision-identical. Reference
  seeds and deterministic scoring match.
- GPU acceleration is deferred; the hardware supports Vulkan/f64 but WGSL does
  not expose f64 and the search is sequential.
- Git branches/commits/issues are prepared in `RUST_REWRITE_TASKLIST.md` but
  were not created or published because the plan reserves that for explicit
  user authorization.
- Deleting the Python tree remains a user decision; it does not affect the
  Rust binary.
