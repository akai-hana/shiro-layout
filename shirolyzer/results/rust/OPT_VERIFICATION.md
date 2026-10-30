# Rust optimization verification

Date: 2026-10-30. Task: OPT.T4.1. This report records the final quality,
parity, determinism, and performance gates on the completed optimization
stack, compared with [`OPT_BASELINE.md`](OPT_BASELINE.md).

## Environment

| Item | Value |
|---|---|
| Host | AMD Ryzen 9 6900HX, 8 cores / 16 threads, 30 GiB RAM |
| OS | Chimera Linux, kernel 7.2.2-0-generic |
| Rust | rustc 1.99.0, release profile (`lto = "thin"`, `codegen-units = 1`) |
| Baseline commit | `9e6e1d7` (`main`, takeover completion) |
| Final commit | `ffbb98e` (`opt-verify`, on top of `opt-prune`) |
| Binary | `shirolyzer/target/release/shirolyzer` |

## Quality gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --release` | pass, including incremental, constrained-dual-delta, thread-invariant table build, and delta/full quench parity tests |
| `check-baseline` | pass, audit 166/166, 9 references |
| `analyze` | pass, primary `3.3940533330472467` |
| `compare` | pass, shiro first at `3.394053` |
| `audit` | pass, probes+checks 37/37, rulings 166/166 |

The analyze primary differs from the frozen baseline
`3.3940533330472595` by `1.3e-14`, well inside the `1e-9` parity tolerance.
The default `compute` selected `;ntgfjoehkrpmscuaydilbqvwxz,./` with primary
`3.520201897371121` and secondary `3.7048525673778605`, matching the baseline.

## Determinism

The same fixed-seed run (`--runs 2 --iters 2000 --ils 2 --polish-rounds 1
--polish-candidates 2 --seed 4242`) was executed with
`RAYON_NUM_THREADS=1` and `RAYON_NUM_THREADS=16`. After removing the elapsed
time and the input-only append path, the JSON documents are identical. The
selected layout and scores match, and the completed run/iteration/ILS/polish
counts are identical.

## Micro-benchmarks

Final `cargo bench --manifest-path shirolyzer/Cargo.toml` output:

```text
tables build: 4.633 ms
delta tables build: 36.737 ms
full score: 17.76 us/op, 56293 ops/s
delta swap: 11639 ns/op, 85921 ops/s
quench: 165.219 ms
full-score quench: 829.694 ms
anneal: 115.884 ms for 10000 iters (86293 iters/s)
```

| Operation | Baseline | Final | Speedup |
|---|---:|---:|---:|
| Delta-table build (parallel) | 41.5 ms | 36.7 ms | 1.13x |
| Full score | 21.1 us/op | 17.8 us/op | 1.19x |
| Single-swap delta | 15,835 ns/op | 11,639 ns/op | 1.36x |
| Delta-scored quench | 221.3 ms | 165.2 ms | 1.34x |
| Full-score quench | 1,006.8 ms | 829.7 ms | 1.21x |
| Anneal | 61,344 iters/s | 86,293 iters/s | 1.41x |

## Short commands

Median of five runs for `check-baseline`, `compare`, and `audit`; median of
seven runs for `analyze`. Output was discarded; seconds.

| Command | Baseline | Final | Speedup |
|---|---:|---:|---:|
| `check-baseline` | 0.036 | 0.0283 | 1.27x |
| `analyze` | 0.045 | 0.0281 | 1.60x |
| `compare` | 0.031 | 0.0168 | 1.85x |
| `audit` | 0.032 | 0.0211 | 1.52x |

The analyzer target was at least 1.3x; the final measurement is 1.60x.

## Generator runs

The small tasklist run (three repetitions) took a median 1.95 s external,
1.83 s internal, versus the 3.92 s baseline: **2.01x** faster. It selected
`;ntgfjoehkrpmscuaydilbqvwxz,./` with primary `3.520201897371121` and
secondary `3.7048525673778605`, exactly the baseline selection.

A full default run (`--runs 28 --iters 200000 --ils 8 --polish-rounds 0`)
completed 37 runs, 10,600,000 iterations, 32 ILS rounds, and 4 secondary
rounds in 30.64 s. It selected the same shiro layout and scores.

## Pruning

No pruning is shipped. The full pair, disjoint two-swap, and three-cycle move
sets remain, and no flag or environment hook was added. The decision and its
measurements are in [`OPT_PRUNING.md`](OPT_PRUNING.md). The
`delta_quench_matches_full_quench` test is the regression gate: the delta
quench and the full-score quench must return the same permutation and score for
a fixed seed.

## Acceptance criteria

| Criterion | Target | Final |
|---|---|---|
| Functional parity | unchanged references, 166/166, nine references | pass |
| Objective parity | within `1e-12` | pass |
| Generator speed | at least 1.5x on the small run | 2.01x |
| Analyzer speed | at least 1.3x | 1.60x |
| Thread-count determinism | identical JSON for 1 and 16 threads | pass |
| Documentation | flags, defaults, pruning decision recorded | pass |
