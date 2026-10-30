# Python vs Rust takeover benchmark

Date: 2026-10-30. Task: RTO.T0.1. This is the pre-takeover performance
baseline comparing the frozen Python oracle with the Rust binary before any
files move or the Python implementation is removed.

## Environment

| Item | Value |
|---|---|
| Host | AMD Ryzen 9 6900HX, 8 cores / 16 threads, 30 GiB RAM |
| OS | Chimera Linux, kernel 7.2.2-0-generic |
| Rust | rustc 1.99.0, release profile (`lto = "thin"`, `codegen-units = 1`) |
| Python | 3.14.6 in `.venv` |
| Python deps | numpy 2.5.4, scipy 1.18.1, PyYAML 6.0.3 |
| Commit | `c558024` (`main`, rust-docs R5.2) |

Both implementations read the same `shirolyzer/config/config.yaml`, the same
parsed corpora, and the same reference layouts. Timing is external wall-clock
time for the whole process, so it includes interpreter startup, imports, table
construction, command execution, and output writes.

## Method

- Seven runs per short command, alternating implementations; the median is
  reported with the full min/max range.
- `compute` is a single larger run because the Python side takes minutes. Both
  sides used identical flags, seed, reference file, and output paths under
  `/tmp`.
- The `compute` append target was a temporary copy of `references.toml`, so the
  tracked reference file was not modified.
- Runs were executed sequentially on an otherwise idle machine after the
  release binary was already built.

Commands:

```bash
.venv/bin/python -m shirolyzer <command>
rust/target/release/shirolyzer <command>
```

The analyzed layout was `;ntgfjoehkrpmscuaydilbqvwxz,./`. The compute run was:

```bash
compute --runs 0 --iters 10 --ils 1 --seed 1234 --max-valid 2 \
  --polish-rounds 0 --append-references /tmp/shirolyzer-bench/refs-X.toml \
  --out /tmp/shirolyzer-bench/X-compute.txt \
  --out-json /tmp/shirolyzer-bench/X-compute.json
```

## Short-command results

Median of seven runs, seconds.

| Command | Python | Rust | Speedup |
|---|---:|---:|---:|
| `check-baseline` | 2.4432 | 0.0351 | **69.6x** |
| `analyze` | 1.1523 | 0.0409 | **28.1x** |
| `compare` | 1.0358 | 0.0279 | **37.2x** |
| `audit` | 1.4011 | 0.0308 | **45.5x** |

Ranges:

| Command | Python min–max | Rust min–max |
|---|---|---|
| `check-baseline` | 2.3784–2.4501 | 0.0347–0.0360 |
| `analyze` | 1.1391–1.1882 | 0.0401–0.0421 |
| `compare` | 1.0309–1.0787 | 0.0274–0.0290 |
| `audit` | 1.3877–1.4480 | 0.0296–0.0315 |

Most of the short-command gap is fixed Python startup/import and NumPy table
construction. The Rust binary still pays table construction, but it is
compiled and much cheaper.

## Compute results

Identical small search on both implementations:

| Command | Python | Rust | Speedup |
|---|---:|---:|---:|
| External wall clock | 240.51 s | 3.75 s | **64.2x** |
| Internal `elapsed_seconds` | 240.16 s | 3.70 s | **64.9x** |

Both runs completed the same work: 9 runs (references only), 110 anneal
iterations, 4 ILS rounds, 2 secondary rounds, 0 polish rounds, and 15 result
entries. Both selected:

```text
layout    ;ntgfjoehkrpmscuaydilbqvwxz,./ (shiro)
primary   3.520201897371
secondary 3.704852567378
```

Score agreement:

| Value | Rust − Python |
|---|---:|
| Selected primary | `-1.38e-14` |
| Selected secondary | `-5.77e-15` |
| Valid-layout count | both 1 |

Both report `selection_status = clear_winner` and the same valid layout. The
only structural JSON difference is the Rust `device: "cpu"` field and some
metadata field ordering.

## Interpretation

- Rust is **28x to 70x faster** on short commands and **about 64x faster** on
  the small end-to-end search.
- The speedup is not from a different workload: phase counts, selected layout,
  valid-layout count, and scores match to floating-point noise.
- For a default compute run the absolute gap is much larger than in the short
  commands because every anneal iteration and ILS round benefits from delta
  scoring, the compiled search loop, and parallel seeds.
- The Python tree is therefore safe to remove from the default path from a
  behavioural standpoint: the Rust binary is both the faster implementation and
  the parity reference for the shipped model. This report is the baseline that
  the post-takeover verification reruns.
