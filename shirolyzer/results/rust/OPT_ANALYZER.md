# Rust analyzer optimization report

Date: 2026-10-30. Task: OPT.T2.4. This report records the analyzer
optimizations from OPT.T2.1 through OPT.T2.3, with before/after timings,
output-diff evidence, and the remaining profile.

## Environment

| Item | Value |
|---|---|
| Host | AMD Ryzen 9 6900HX, 8 cores / 16 threads, 30 GiB RAM |
| OS | Chimera Linux, kernel 7.2.2-0-generic |
| Rust | rustc 1.99.0, release profile (`lto = "thin"`, `codegen-units = 1`) |
| Baseline commit | `9e6e1d7` (`main`, takeover completion) |
| Optimized commit | `cf65267` (`opt-analyzer`, OPT.T2.3) |
| Binary | `shirolyzer/target/release/shirolyzer` |

## Changes

- OPT.T2.1: built independent bigram/trigram table rows in parallel and added
  reusable `penalty_mask_into` / `tri_many_into` paths so table construction no
  longer allocates a fresh mask per call.
- OPT.T2.2: ran the category, trigram-pattern, finger-usage, and
  fingering-guide passes concurrently with `rayon::join`. The fingering guide
  stores contexts as `[char; 3]` and only formats the surviving top entries
  instead of building a `String` for every corpus trigram.
- OPT.T2.3: read the three parsed corpus files concurrently and switched the
  report loops to the numeric `bi_ids`/`tri_ids` vectors with a fixed
  letter-index keymap.

## Short-command timings

No CPU pinning, no Rayon cap, default thread pool. Eleven interleaved runs
against the baseline binary, first run discarded; seconds.

| Binary | Median | Range | Speedup |
|---|---:|---:|---:|
| Baseline `9e6e1d7` | 0.0433 | 0.0424-0.0452 | 1.00x |
| Optimized `cf65267` | 0.0262 | 0.0248-0.0285 | **1.65x** |

The tasklist target is at least 1.3x; the measured speedup is 1.65x.

Pinned four-thread A/B runs (taskset cores 0-3, `RAYON_NUM_THREADS=4`) isolate
each task:

| Commit | Median | Change |
|---|---:|---:|
| `9e32bd7` (after T2.1) | 0.0346 | - |
| `fcaba14` (after T2.2) | 0.0305 | 1.13x |
| `cf65267` (after T2.3) | 0.0314 | within run noise of T2.2 |

T2.3 is timing-neutral on this small corpus: the files are only 176 KiB, so the
parallel read saves little, while removing the per-entry `HashMap<char, _>`
lookups offsets that overhead. It is retained because it removes the allocation
and hashing work from the hot report loops.

## Output parity

The baseline and optimized binaries were run with the shipped config and the
default `shiro` layout:

```bash
shirolyzer/target/release/shirolyzer analyze \
  --out /tmp/unpinned-base.txt --out-json /tmp/unpinned-base.json
# same command with the optimized binary, into unpinned-opt.{txt,json}
```

- Text output: `diff` is empty.
- JSON output: parsed documents compare equal, including scores, categories,
  trigram patterns, usage, worst bigrams, scissors, and guide counts.
- `check-baseline`: `baseline OK: audit 166/166, 9 references`.

## Remaining profile

`perf record -F 9999 -g --call-graph dwarf` over ten analyze runs, pinned to
four cores with `RAYON_NUM_THREADS=4`:

| Symbol | Overhead |
|---|---:|
| `model::Model::tri_many_into` | 24.9% |
| `gates::Gates::penalty_mask_into` | 17.4% |
| `analyze::fingering_guide` | 5.6% |
| SipHash writes | 5.3% |
| Hashing `(char, char, char)` | 3.3% |
| Rayon table-build bridge | 2.9% |
| `memset` | 2.2% |
| Page allocation | 2.2% |
| `Corpus::new` trigram sort | 2.0% |

The profile has moved from the report passes to `Tables::build`: the two model
and gate helpers now account for about 42% of an analyze run. That work is
shared by every command, so further analyzer-only gains would need either a
cheaper table build or a cached on-disk table. Both are model-level changes and
outside this epic's no-objective-change constraint; they belong in a separate
plan if wanted.

## Gates

- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.
- `cargo test --release` passes, including command, baseline, parity, and
  randomized delta-vs-full scoring tests.
- Analyze primary score and categories are unchanged within `1e-9`.
