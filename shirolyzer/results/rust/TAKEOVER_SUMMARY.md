# Rust takeover summary

Date: 2026-10-30. Status: complete and published. Tag: `rust-takeover`.

This epic made the Rust implementation the only `shirolyzer` runtime, moved the
crate into `shirolyzer/`, and removed the Python runtime while retaining the
corpus parsers and model-refitting tools.

## Outcome

- The runtime is `shirolyzer/target/release/shirolyzer`, built from
  `shirolyzer/Cargo.toml`.
- The Python runtime, CLI facades, `commands/`, and Python runtime tests are
  gone from `main`.
- The corpus parsers (`corpora/*/parse_*.py`) and model-refitting tools
  (`shirolyzer/fitting/`) remain as development-only tools. They keep the
  Python support modules they need, including `config.py` and `conditions.py`.
- Config, corpus, baseline, and result paths did not move. The objective,
  defaults, audit rulings, and command outputs are unchanged.
- Historical Python code is still available through git and the pre-takeover
  commits/tags; it is not part of the current `main` tree.

## Branches and issues

| Branch | Task(s) | Issue(s) | Final commit |
|---|---|---|---|
| `takeover-bench` | RTO.T0.1 | #66 | `f35ea75` |
| `takeover-plan` | RTO.T0.2 | #67 | `0eacf6d` |
| `takeover-move` | RTO.T1.1, RTO.T1.2 | #68, #69 | `9b633af` |
| `takeover-python` | RTO.T2.1, RTO.T2.2 | #70, #71 | `c5a3f91` |
| `takeover-tooling` | RTO.T3.1, RTO.T3.2 | #72, #73 | `3d1244d` |
| `takeover-verify` | RTO.T4.1, RTO.T4.2 | #74, #75 | this commit |

The branches are stacked in that order. `main` was fast-forwarded to the
`takeover-verify` head after the final gates passed, and every branch was
pushed to `origin`.

## Quality gates

All gates ran from the new layout:

| Gate | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test --release` | 9 passed, 1 intentional ignore, 0 failed |
| `check-baseline` | audit 166/166, 9 references |
| `analyze` | `primary=3.394053333`, `secondary=3.494871416` |
| `compare` | shiro first at `3.394053` |
| `audit` | rulings 166/166, probes/checks 37/37 |
| Dev tools | `compileall`, fitting imports, and model tables pass |

## Performance

Post-takeover medians of seven runs are within a few milliseconds of the
pre-takeover Rust medians; the small compute run is slightly faster and selects
the same layout:

| Run | Pre-takeover Rust | Post-takeover Rust |
|---|---:|---:|
| `check-baseline` | 0.0351 s | 0.036 s |
| `analyze` | 0.0409 s | 0.044 s |
| `compare` | 0.0279 s | 0.030 s |
| `audit` | 0.0308 s | 0.032 s |
| Small compute | 3.75 s | 3.678 s |

The compute run selected `;ntgfjoehkrpmscuaydilbqvwxz,./` with primary
`3.520201897371` and secondary `3.704852567378`, the same values as before the
takeover. Full methods, ranges, and gate output are in
[`TAKEOVER_VERIFICATION.md`](TAKEOVER_VERIFICATION.md); the pre-takeover
baseline is [`TAKEOVER_BENCHMARKS.md`](TAKEOVER_BENCHMARKS.md).

## Python-removal decision

The Python runtime was removed because the Rust binary matched its reference,
analyze, audit, and baseline behaviour before the takeover, and was 28–70x
faster on short commands and about 64x faster on the small compute comparison.
The remaining Python code is limited to development workflows that the runtime
does not need: building parsed corpora and refitting the pattern model. Removing
those tools was out of scope; they can be ported or retired separately.

## Rollback

`git checkout rust-takeover^` or the pre-takeover backup tags recover the
Python runtime tree. The Python implementation is never deleted from history;
only the current `main` tree is Rust-only.
