# Rust generator optimization report

Date: 2026-10-30. Task: OPT.T1.4. This report records the generator
optimizations from OPT.T1.1 through OPT.T1.3, with before/after measurements,
determinism evidence, and score/selection parity.

## Environment

| Item | Value |
|---|---|
| Host | AMD Ryzen 9 6900HX, 8 cores / 16 threads, 30 GiB RAM |
| OS | Chimera Linux, kernel 7.2.2-0-generic |
| Rust | rustc 1.99.0, release profile (`lto = "thin"`, `codegen-units = 1`) |
| Baseline commit | `9e6e1d7` (`main`, takeover completion) |
| Optimized commit | `3d74ff8` (`opt-generator`, OPT.T1.3) |
| Binary | `shirolyzer/target/release/shirolyzer` |

The benchmark table was collected before background video playback began.
Remaining verification runs used `nice -n 19` and a reduced Rayon thread cap.

## Changes

- OPT.T1.1: replaced full rescoring in the constrained secondary phase with a
  dual-delta quench (`search::quench_constrained`). The move order and RNG
  consumption mirror `quench_generic`; a randomized parity test matches the
  full-score path exactly.
- OPT.T1.2: parallelized ILS rows and polish candidates. Each candidate gets a
  deterministic RNG stream derived from its position, so fixed seeds produce
  identical results at any Rayon thread count. Time-budgeted runs stay
  sequential and use the same per-candidate streams.
- OPT.T1.3: stored key and trigram ids as `u8`, copied smaller arrays in
  `Local::delta_cross`, replaced the cross-table
  `HashMap<[u8; 4], Vec<u16>>` with a dense move-indexed `Vec<Vec<u16>>`, and
  reused the scratch buffer when building the table.

## Micro-benchmarks

`cargo bench --manifest-path shirolyzer/Cargo.toml`:

| Operation | Baseline | Optimized | Speedup |
|---|---:|---:|---:|
| Delta-table build (parallel) | 41.5 ms | 38.0 ms | 1.09x |
| Full score | 21.1 us/op | 20.0 us/op | 1.06x |
| Single-swap delta | 15,835 ns/op | 11,952 ns/op | 1.32x |
| Delta-scored quench | 221.3 ms | 171.8 ms | 1.29x |
| Full-score quench (`quench_generic`) | 1,006.8 ms | 911.6 ms | 1.10x |
| Anneal, 10k iterations | 163.0 ms, 61,344 iters/s | 113.9 ms, 87,773 iters/s | 1.43x |

The constrained secondary phase no longer calls the full-score quench, so the
remaining `quench_generic` time is a fallback path rather than the hot
secondary path.

## Small compute run

```bash
shirolyzer/target/release/shirolyzer compute \
  --runs 0 --iters 10 --ils 1 --seed 1234 --max-valid 2 \
  --polish-rounds 0 \
  --append-references /tmp/shiro-opt-baseline/refs.toml \
  --out /tmp/shiro-opt-baseline/c5-final.txt \
  --out-json /tmp/shiro-opt-baseline/c5-final.json
```

| Value | Baseline | Optimized |
|---|---:|---:|
| External wall clock | 3.92 s | 1.73 s |
| Internal `elapsed_seconds` | 3.883 s | 1.725 s |
| Selected layout | `;ntgfjoehkrpmscuaydilbqvwxz,./` | `;ntgfjoehkrpmscuaydilbqvwxz,./` |
| Selected primary | 3.520201897371 | 3.520201897371 |
| Selected secondary | 3.704852567378 | 3.704852567378 |
| Completed runs / iterations / ILS / secondary | 9 / 110 / 4 / 2 | 9 / 110 / 4 / 2 |

## Determinism

The same fixed-seed run was executed with one and sixteen Rayon threads:

```bash
RAYON_NUM_THREADS=1  shirolyzer/target/release/shirolyzer compute \
  --runs 2 --iters 2000 --ils 2 --polish-rounds 1 --polish-candidates 2 \
  --seed 4242 --append-references /tmp/shiro-det/refs1.toml \
  --out /tmp/shiro-det/out1.txt --out-json /tmp/shiro-det/det1.json

RAYON_NUM_THREADS=16 shirolyzer/target/release/shirolyzer compute \
  --runs 2 --iters 2000 --ils 2 --polish-rounds 1 --polish-candidates 2 \
  --seed 4242 --append-references /tmp/shiro-det/refs16.toml \
  --out /tmp/shiro-det/out16.txt --out-json /tmp/shiro-det/det16.json
```

After removing `elapsed_seconds` and the input-only `append_references` path,
the two JSON documents are identical. The selected layouts, scores, result
order, and all round counts match.

## Gates and parity

- `cargo test --release`: all unit, parity, baseline, and command tests pass.
- `shirolyzer check-baseline`: `baseline OK: audit 166/166, 9 references`.
- `analyze`: primary `3.3940533330472467` versus baseline
  `3.3940533330472595`, a difference of `1.3e-14`; the baseline test tolerance
  passes.
- Small compute selected layout and both scores match the baseline exactly.
- `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` pass.
