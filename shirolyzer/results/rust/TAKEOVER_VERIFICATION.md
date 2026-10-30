# Post-takeover verification

Date: 2026-10-30. Task: RTO.T4.1. This report reruns the quality gates and the
takeover benchmark after the crate moved from `rust/` into `shirolyzer/` and
the Python runtime was removed.

## Environment

| Item | Value |
|---|---|
| Host | AMD Ryzen 9 6900HX, 8 cores / 16 threads, 30 GiB RAM |
| OS | Chimera Linux, kernel 7.2.2-0-generic |
| Rust | rustc 1.99.0, release profile (`lto = "thin"`, `codegen-units = 1`) |
| Python | 3.14.6 in `.venv` (development tools only) |
| Branch | `takeover-verify` from `3d1244d` |
| Binary | `shirolyzer/target/release/shirolyzer` |

The pre-takeover numbers are from
[`TAKEOVER_BENCHMARKS.md`](TAKEOVER_BENCHMARKS.md), measured on the same host at
`c558024` with the binary at `rust/target/release/shirolyzer`.

## Quality gates

All gates pass from the new layout:

```bash
cargo fmt --manifest-path shirolyzer/Cargo.toml --check
cargo clippy --manifest-path shirolyzer/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path shirolyzer/Cargo.toml --release
shirolyzer/target/release/shirolyzer check-baseline
shirolyzer/target/release/shirolyzer analyze ";ntgfjoehkrpmscuaydilbqvwxz,./" \
  --out /tmp/rs-analysis.txt --out-json /tmp/rs-analysis.json
shirolyzer/target/release/shirolyzer compare
shirolyzer/target/release/shirolyzer audit \
  --out /tmp/rs-audit.txt --out-json /tmp/rs-audit.json
```

Results:

- `cargo fmt --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --release`: 9 passed, 1 ignored (`compute_smoke`, intentional),
  0 failed.
- `check-baseline`: `baseline OK: audit 166/166, 9 references`.
- `audit`: `rulings: 166/166`; probes and hard checks `37/37`.
- `analyze`: `primary=3.394053333`, `secondary=3.494871416`.
- `compare`: shiro ranks first at `3.394053`.
- Retained dev tools: `compileall`, `shirolyzer.fitting` imports, and
  `comfort.Model().tables()` all succeed.

## Short commands

Median of seven runs, seconds, with command output and stderr discarded so only
process time is measured.

| Command | Pre-takeover Rust | Post-takeover Rust | Delta |
|---|---:|---:|---:|
| `check-baseline` | 0.0351 | 0.036 | +0.0009 |
| `analyze` | 0.0409 | 0.044 | +0.0031 |
| `compare` | 0.0279 | 0.030 | +0.0021 |
| `audit` | 0.0308 | 0.032 | +0.0012 |

Ranges:

| Command | Post-takeover min–max |
|---|---|
| `check-baseline` | 0.036–0.040 |
| `analyze` | 0.042–0.048 |
| `compare` | 0.028–0.031 |
| `audit` | 0.031–0.034 |

The deltas are small and within normal run-to-run drift; there is no new
runtime dependency, the binary reads the same config/corpora, and command
outputs match the pre-takeover results.

## Compute

Identical small search, same flags and seed as the pre-takeover report:

```bash
shirolyzer/target/release/shirolyzer compute \
  --runs 0 --iters 10 --ils 1 --seed 1234 --max-valid 2 \
  --polish-rounds 0 \
  --append-references /tmp/shirolyzer-post/refs-post.toml \
  --out /tmp/shirolyzer-post/post-compute.txt \
  --out-json /tmp/shirolyzer-post/post-compute.json
```

| Value | Pre-takeover | Post-takeover |
|---|---:|---:|
| External wall clock | 3.75 s | 3.678 s |
| Internal `elapsed_seconds` | 3.70 s | 3.647 s |
| Selected layout | `;ntgfjoehkrpmscuaydilbqvwxz,./` | same |
| Selected primary | 3.520201897371 | 3.520201897371 |
| Selected secondary | 3.704852567378 | 3.704852567378 |
| Valid layouts | 1 | 1 |
| Selection status | `clear_winner` | `clear_winner` |
| Runs / iterations / ILS / secondary | 9 / 110 / 4 / 2 | 9 / 110 / 4 / 2 |

The selected layout, scores, valid-layout count, and phase counts are unchanged.
The score agreement with the pre-takeover run is at floating-point noise
(`< 1e-14`).

## Interpretation

- Moving the crate and removing the Python runtime did not change the objective,
  defaults, config/data paths, or command outputs.
- Short commands remain in the same few-millisecond range; the small increase is
  measurement drift, not a new cost.
- The small compute run completes slightly faster than the pre-takeover run and
  reproduces the same selected layout and scores.
- The runtime is the Rust binary only; the retained Python code is limited to
  the corpus parsers and model-refitting tools.
