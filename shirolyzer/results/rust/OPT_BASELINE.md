# Rust optimization baseline

Date: 2026-10-30. Task: OPT.T0.1. This report freezes the pre-optimization
performance and profiling evidence used by `RUST_OPTIMIZATION_TASKLIST.md`.

## Environment

| Item | Value |
|---|---|
| Host | AMD Ryzen 9 6900HX, 8 cores / 16 threads, 30 GiB RAM |
| OS | Chimera Linux, kernel 7.2.2-0-generic |
| Rust | rustc 1.99.0, release profile (`lto = "thin"`, `codegen-units = 1`) |
| Commit | `9e6e1d7` (`main`, takeover completion) |
| Binary | `shirolyzer/target/release/shirolyzer` |

## Micro-benchmarks

`cargo bench --manifest-path shirolyzer/Cargo.toml`:

| Operation | Baseline |
|---|---:|
| Delta-table build (parallel) | 41.5 ms |
| Full score | 21.1 us/op, 47,322 ops/s |
| Single-swap delta | 15,835 ns/op, 63,150 ops/s |
| Delta-scored quench | 221.3 ms |
| Full-score quench (`quench_generic`) | 1,006.8 ms |
| Anneal, 10k iterations | 163.0 ms, 61,344 iters/s |

The full-score quench is 4.6x slower than the delta-scored quench. It is still
used by the constrained secondary phase, so that phase has a known 4x headroom.

## Short commands

Median of five runs, seconds, with output discarded. Analyze used the shipped
`shiro` layout.

| Command | Baseline median | Range |
|---|---:|---:|
| `check-baseline` | 0.036 | 0.036-0.039 |
| `analyze` | 0.045 | 0.044-0.046 |
| `compare` | 0.031 | 0.030-0.031 |
| `audit` | 0.032 | 0.032-0.035 |

## Small compute run

Identical run used by the takeover reports:

```bash
shirolyzer/target/release/shirolyzer compute \
  --runs 0 --iters 10 --ils 1 --seed 1234 --max-valid 2 \
  --polish-rounds 0 \
  --append-references /tmp/shiro-opt-baseline/refs.toml \
  --out /tmp/shiro-opt-baseline/compute.txt \
  --out-json /tmp/shiro-opt-baseline/compute.json
```

| Value | Baseline |
|---|---:|
| External wall clock | 3.92 s |
| Internal `elapsed_seconds` | 3.883 s |
| Selected layout | `;ntgfjoehkrpmscuaydilbqvwxz,./` |
| Selected primary | 3.520201897371 |
| Selected secondary | 3.704852567378 |
| Completed runs / iterations / ILS / secondary | 9 / 110 / 4 / 2 |

## Profiles

`perf record -F 999 -g --call-graph dwarf` on the commands above.

### Small compute

| Symbol | Overhead |
|---|---:|
| `search::Local::delta_cross` | 61.7% |
| `search::Local::delta` | 28.9% |
| `search::DeltaTables::build` | 3.8% |
| `memcpy` | 2.5% |

The generator is dominated by the two-pair cross-delta path: it copies three
`[usize; 26]` arrays per candidate and looks up the cross list through a
`HashMap<[u8; 4], Vec<u16>>`.

### Analyze

| Symbol | Overhead |
|---|---:|
| `model::Model::tri_many` | 21.8% |
| `analyze::fingering_guide` | 15.0% |
| `memset` | 6.1% |
| Page-fault/allocation path | 4.5% |

`Tables::build` (through `tri_many`) and the allocation-heavy fingering guide
are the largest analyzer costs. The three report passes are independent and
currently run sequentially.

## Baseline gates

The takeover verification already records the functional gates at this commit:
`check-baseline` reports `audit 166/166, 9 references`, analyze primary is
`3.394053333`, and all Rust tests pass. Optimization must not change these
values.
