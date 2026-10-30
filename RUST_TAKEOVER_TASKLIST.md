# Shirolyzer Rust Takeover — Tasklist and Implementation Plan

Status: in progress. This is the follow-up epic to `RUST_REWRITE_TASKLIST.md`.
The rewrite is complete; this plan makes the Rust implementation the only
runtime implementation and moves it into `shirolyzer/`.

Source history: `RUST_REWRITE_SPEC_LIST.txt` remains unchanged. The
pre-takeover benchmark is
[`shirolyzer/results/rust/TAKEOVER_BENCHMARKS.md`](shirolyzer/results/rust/TAKEOVER_BENCHMARKS.md).
Last updated: 2026-10-30.

---

## 1. Locked decisions

1. **One runtime implementation.** The Rust binary is the only supported
   `shirolyzer` runtime. Python is removed from the default path.
2. **Crate location.** The Rust crate moves from `rust/` into `shirolyzer/`.
   The final tracked layout is:
   - `shirolyzer/Cargo.toml`, `Cargo.lock`
   - `shirolyzer/src/`, `shirolyzer/tests/`, `shirolyzer/benches/`
   - `shirolyzer/config/`, `shirolyzer/results/` (unchanged data paths)
   - `shirolyzer/fitting/` and the Python modules it needs (development only)
   - `corpora/*/parse_*.py` (development/build-time corpus parsers)
3. **Python disposition.** The Python runtime, CLI facades, `commands/`, and
   Python runtime tests are deleted from `main` in a dedicated commit. History
   before the takeover remains available through git and the `rust-takeover`
   tag.
4. **Development tools retained.** Corpus parsers and the model-refitting tools
   are non-runtime development tools and stay Python. They are not documented
   as `shirolyzer` runtime commands and are not part of the release binary.
5. **Compatibility.** CLI names, flags, defaults, config/data paths, and JSON
   output remain compatible with the Rust rewrite. The shipped binary is still
   named `shirolyzer` and still runs from the repository root or from outside
   the repository.
6. **Git workflow.** One task equals one commit; commit messages use the branch
   prefix (`takeover-move: RTO.T1.1 — ...`). Branches are stacked in the order
   below and merged into `main` by fast-forward only after the exit conditions
   pass. Every branch and issue is pushed/published as part of the epic.
7. **No score changes.** This takeover is a packaging and lifecycle change. It
   must not alter the objective, defaults, corpus data, or audit rulings.
8. **No unreviewed deletions.** Only Python files identified as runtime or
   runtime tests are removed. Development tools stay until a separate plan
   ports or retires them.

## 2. Baseline evidence

The pre-takeover comparison (`c558024`, Python 3.14.6 / Rust 1.99.0) found:

| Command | Python median | Rust median | Speedup |
|---|---:|---:|---:|
| `check-baseline` | 2.4432 s | 0.0351 s | 69.6x |
| `analyze` | 1.1523 s | 0.0409 s | 28.1x |
| `compare` | 1.0358 s | 0.0279 s | 37.2x |
| `audit` | 1.4011 s | 0.0308 s | 45.5x |
| small `compute` | 240.51 s | 3.75 s | 64.2x |

The small compute run completed the same phases, selected the same layout, and
matched primary/secondary scores within `1e-14`. Full method and raw ranges are
in `shirolyzer/results/rust/TAKEOVER_BENCHMARKS.md`.

## 3. Git roadmap

Branches are stacked from `main` in this order. Each row's base is the previous
row's completed branch.

| Order | Branch | Scope | Exit condition |
|---|---|---|---|
| 0 | `takeover-bench` | Python-vs-Rust benchmark report | Report committed; measurements reproducible |
| 1 | `takeover-plan` | This tasklist, milestone, labels, issues | Plan committed; issues #66–#75 published |
| 2 | `takeover-move` | Move the crate to `shirolyzer/`; adapt paths/build | Build/tests/baseline pass at the new location |
| 3 | `takeover-python` | Delete Python runtime/tests; retain dev tools | No runtime Python remains; retained tools import |
| 4 | `takeover-tooling` | justfile, READMEs, repository map, stale refs | Documented commands match the new layout |
| 5 | `takeover-verify` | Final gates, post-move benchmark, completion, tag | All gates pass; `main` fast-forwards; tag published |

Published issues (milestone `Rust takeover`; labels `rewrite`, `rust`,
`takeover`, and the branch label):

| Issue | Branch | Task |
|---|---|---|
| [#66](https://github.com/akai-hana/shiro-layout/issues/66) | `takeover-bench` | RTO.T0.1 — Benchmark Python versus Rust before takeover |
| [#67](https://github.com/akai-hana/shiro-layout/issues/67) | `takeover-plan` | RTO.T0.2 — Write takeover tasklist and publish roadmap |
| [#68](https://github.com/akai-hana/shiro-layout/issues/68) | `takeover-move` | RTO.T1.1 — Relocate the Rust crate into shirolyzer/ |
| [#69](https://github.com/akai-hana/shiro-layout/issues/69) | `takeover-move` | RTO.T1.2 — Update path and build assumptions for the moved crate |
| [#70](https://github.com/akai-hana/shiro-layout/issues/70) | `takeover-python` | RTO.T2.1 — Remove the Python runtime, CLI, facades, and tests |
| [#71](https://github.com/akai-hana/shiro-layout/issues/71) | `takeover-python` | RTO.T2.2 — Retain and verify the corpus parsers and fitting dev tools |
| [#72](https://github.com/akai-hana/shiro-layout/issues/72) | `takeover-tooling` | RTO.T3.1 — Update justfile, build paths, and install instructions |
| [#73](https://github.com/akai-hana/shiro-layout/issues/73) | `takeover-tooling` | RTO.T3.2 — Update READMEs, repository map, and stale references |
| [#74](https://github.com/akai-hana/shiro-layout/issues/74) | `takeover-verify` | RTO.T4.1 — Run post-takeover gates and benchmark verification |
| [#75](https://github.com/akai-hana/shiro-layout/issues/75) | `takeover-verify` | RTO.T4.2 — Record completion, tag, and publish takeover branches |

Branch rule: one task equals one commit unless a task is explicitly a single
file move plus its path fix. Do not rewrite or edit
`RUST_REWRITE_SPEC_LIST.txt`. Do not delete development tools in this epic.

## 4. Quality gates

Run from the repository root after the move; the binary path becomes
`shirolyzer/target/release/shirolyzer`.

```bash
cargo fmt --manifest-path shirolyzer/Cargo.toml --check
cargo clippy --manifest-path shirolyzer/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path shirolyzer/Cargo.toml --release
shirolyzer/target/release/shirolyzer check-baseline
shirolyzer/target/release/shirolyzer analyze ";ntgfjoehkrpmscuaydilbqvwxz,./" --out /tmp/rs-analysis.txt --out-json /tmp/rs-analysis.json
shirolyzer/target/release/shirolyzer compare
shirolyzer/target/release/shirolyzer audit --out /tmp/rs-audit.txt --out-json /tmp/rs-audit.json
```

Regression gates:

1. **Baseline:** audit 166/166 and all nine references within `1e-9`.
2. **Objective:** selected layout/scores unchanged from the pre-takeover
   benchmark.
3. **Docs:** every command in `README.md`, `shirolyzer/README.md`, and
   `justfile` runs as written.
4. **Dev tools:** retained Python modules import; corpus parser `--check`
   paths are documented and remain runnable.
5. **No runtime Python:** no `python -m shirolyzer`, Python CLI facade, or
   runtime command remains.

Performance gate: rerun the `TAKEOVER_BENCHMARKS.md` short commands and a small
`compute` on the moved binary; record the post-move numbers and compare them
with the pre-takeover baseline.

## 5. Tasks

### Branch `takeover-bench`

- [x] **RTO.T0.1 — Benchmark Python versus Rust before takeover**
  - Files: `shirolyzer/results/rust/TAKEOVER_BENCHMARKS.md`.
  - Record environment, method, short-command medians, identical compute run,
    and score/selection parity.
  - Done when the report is committed on `takeover-bench`.

### Branch `takeover-plan`

- [x] **RTO.T0.2 — Write takeover tasklist and publish roadmap**
  - Files: `RUST_TAKEOVER_TASKLIST.md`.
  - Publish milestone `Rust takeover`, labels, issues #66–#75, and the branch
    mapping above.
  - Done when the plan is committed and all issues are visible on GitHub.

### Branch `takeover-move`

- [x] **RTO.T1.1 — Relocate the Rust crate into shirolyzer/**
  - Move `rust/Cargo.toml`, `Cargo.lock`, `src/`, `tests/`, and `benches/` into
    `shirolyzer/`; remove the now-empty `rust/` tree.
  - Keep `shirolyzer/config/`, `shirolyzer/results/`, `shirolyzer/fitting/`,
    and the retained Python modules in place.
  - Done when git records the moves and the crate builds/tests from its new
    location.
  - Issue: #68.

- [x] **RTO.T1.2 — Update path and build assumptions for the moved crate**
  - Update `shirolyzer/src/paths.rs` root discovery and manifest fallback for
    the new `shirolyzer/` manifest location.
  - Update any tests, bench paths, or build scripts that mention `rust/`.
  - Done when the binary finds `shirolyzer/config/config.yaml` from the repo
    root and from outside the repo, and all gates pass.
  - Issue: #69.

### Branch `takeover-python`

- [x] **RTO.T2.1 — Remove the Python runtime, CLI, facades, and tests**
  - Delete `shirolyzer/__main__.py`, `shirolyzer/analyze.py`,
    `shirolyzer/compute.py`, `shirolyzer/commands/`, and `shirolyzer/tests/`
    Python tests.
  - Delete runtime-only utility modules not imported by `shirolyzer/fitting/`
    (expected: `corpus.py`, `diversity.py`, `references.py`, `render.py`,
    `scorer.py`, `search.py`, `xkb.py`). `config.py` and `conditions.py` stay
    because the retained `comfort`/`gates`/`fitting` tables import them.
  - Update the `shirolyzer` package docstring so it no longer advertises a
    Python runtime.
  - Done when no Python runtime entry point remains and `rg` finds no runtime
    command referencing `python -m shirolyzer`.
  - Issue: #70.

- [x] **RTO.T2.2 — Retain and verify the corpus parsers and fitting dev tools**
  - Keep `corpora/*/parse_*.py`, `shirolyzer/fitting/`, and the Python modules
    they import (`comfort`, `config`, `conditions`, `fitlib`, `gates`, `gateset`,
    `rulings`, `paths`, `fingers`, `ansi`).
  - Trim `config.apply` down to the dev-tool state and drop dead corpus loading
    so it no longer imports deleted runtime modules.
  - Verify `shirolyzer.fitting` imports, table construction works, and the
    corpus parser `--help`/`--check` paths still work; document that these are
    development-only tools.
  - Done when retained tools import cleanly and the docs call them dev tools.
  - Issue: #71.

### Branch `takeover-tooling`

- [x] **RTO.T3.1 — Update justfile, build paths, and install instructions**
  - Point `justfile` at `shirolyzer/Cargo.toml` and
    `shirolyzer/target/release/shirolyzer`.
  - Replace the Python-oracle `parity` recipe with the Rust baseline/check
    recipe; remove or repurpose `py-test`.
  - Done when `just build`, `just test`, `just baseline`, and `just check` work
    from a clean checkout of the new layout.
  - Issue: #72.

- [x] **RTO.T3.2 — Update READMEs, repository map, and stale references**
  - Update root `README.md`, `shirolyzer/README.md`, `RUST_REWRITE_TASKLIST.md`
    completion notes, and links to result reports.
  - Remove stale `rust/target`, Python runtime, and deleted-module instructions;
    document retained development tools separately.
  - Done when every documented command runs as written and every local link
    resolves.
  - Issue: #73.

### Branch `takeover-verify`

- [x] **RTO.T4.1 — Run post-takeover gates and benchmark verification**
  - Run the full quality gates and the takeover benchmark from the new layout.
  - Compare selected layout/scores and short-command timings with the
    pre-takeover report; record both.
  - Done when all gates pass and the post-move numbers are committed.
  - Issue: #74.

- [ ] **RTO.T4.2 — Record completion, tag, and publish takeover branches**
  - Write `shirolyzer/results/rust/TAKEOVER_SUMMARY.md`, mark this tasklist
    complete, and create annotated tag `rust-takeover`.
  - Push all six branches, close issues #66–#75, and fast-forward `main` to the
    final takeover commit.
  - Done when the epic is published and `main` contains only the Rust runtime.
  - Issue: #75.

## 6. Rollback

Before the tag, any branch can be abandoned; `main` still points at the
pre-takeover `c558024` tree. After the tag, `git checkout rust-takeover^` or the
pre-takeover `backup/pre-rust-publish` tag recovers the Python runtime. The
Python tree is never deleted from history, only from the current `main` tree.

## 7. Progress log

- 2026-10-30: Benchmarked the frozen Python oracle against the Rust binary.
  Rust is 28–70x faster on short commands and 64x faster on a small compute
  run, with matching scores and selected layouts.
- 2026-10-30: Wrote this takeover plan, created milestone `Rust takeover`,
  labels, issues #66–#75, and started the stacked branch sequence.
- 2026-10-30: Moved the crate to `shirolyzer/` and adapted the path discovery,
  build commands, tests, and benchmarks. The binary builds and all Rust gates
  pass from the new location.
- 2026-10-30: Removed the Python runtime, CLI facades, command modules, and
  Python runtime tests. Kept `config.py` and `conditions.py` as dev-tool
  support after trimming `config.apply` so the retained refitting tables do not
  import deleted search/scorer modules. Verified the corpus parsers, fitting
  imports, and table construction still work.
- 2026-10-30: Pointed `justfile` at the moved manifest and binary, replaced the
  Python parity recipe with the Rust baseline, and added a `dev-tools` recipe
  for the retained Python tooling. Rewrote the root and analyzer READMEs for
  the new layout, removed the Python-runtime instructions, mapped the retained
  development tools separately, and added a takeover note to the rewrite
  completion record.
- 2026-10-30: Reran all quality gates and the takeover benchmark from the new
  layout. All gates pass, the selected layout and scores are unchanged, and
  the small compute run is slightly faster. Recorded both in
  `shirolyzer/results/rust/TAKEOVER_VERIFICATION.md`.
