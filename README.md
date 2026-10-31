# shiro【白】

Shiro is a 30-key alternate keyboard layout designed around
context-dependent fingering.  This repository contains **shirolyzer**, the
analyzer and generator used to evaluate and iterate the layout, plus the
XKB symbols and installer needed to try a generated arrangement on X11.

## What is here

- **Calibrated pattern model** — scores layouts against corpus bigrams,
  trigrams, geometry, comfort terms, condition rules, and finger balance.
- **Analyzer** — detailed bigram/trigram, scissor, finger-load, and fingering
  reports for any 30-character layout.
- **Auditor** — questionnaire probes, hard checks, and model rulings.
- **Generator** — primary search, ILS, optional polishing, constrained
  secondary-corpus tuning, and diversity-aware valid-layout reporting.
- **Reference comparison** — ranks the layouts listed in a TOML reference file.
- **XKB workflow** — converts a layout string into symbols and installs it.

The primary implementation is the Rust binary in `rust/` (crate
`shirolyzer`).  It provides `compute`, `analyze`, `render`, `xkb`, `compare`,
`audit`, and `check-baseline`.  The frozen Python package under `shirolyzer/`
remains the parity oracle and still contains the corpus parsers and
model-refitting tools.  The rewrite plan and results are in
`RUST_REWRITE_TASKLIST.md` and `shirolyzer/results/rust/`.

## Quick start

Requires Rust 1.99+ for the analyzer and Python 3.11+ with the dependencies in
`requirements.txt` for the corpus parsers and the parity oracle.

```bash
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt
cargo build --manifest-path rust/Cargo.toml --release
```

Build the parsed corpora (network access is required):

```bash
python3 corpora/shai/parse_shai.py
python3 corpora/spanish/parse_spanish.py
```

Analyze a layout:

```bash
rust/target/release/shirolyzer analyze ";ntgfjoehkrpmscuaydilbqvwxz,./"
```

Render it as a staggered ASCII grid with its configured fingers:

```bash
rust/target/release/shirolyzer render ";ntgfjoehkrpmscuaydilbqvwxz,./"
```

Generate a layout and write it to XKB:

```bash
rust/target/release/shirolyzer compute --name shiro --xkb --xkb-name shiro
```

`compute --device cpu` is the default and the only shipped backend;
`--device gpu` currently falls back to CPU with a warning (see
`shirolyzer/results/rust/GPU_FEASIBILITY.md`).  A default compute run is a
long-running search; see [`shirolyzer/README.md`](shirolyzer/README.md) for
shorter experiments and for the meaning of `selected` versus the full
`valid_layouts` list.

If you use [`just`](https://github.com/casey/just), the common workflows are
shortcuts:

```bash
just compute
just analyze ";ntgfjoehkrpmscuaydilbqvwxz,./"
just render ";ntgfjoehkrpmscuaydilbqvwxz,./"
just test
just parity
```

Install the generated symbols on the current X display:

```bash
xkb/install.sh shiro
```

`xkb/install.sh` with no arguments installs the shipped `xkb/mine` layout as
`mine`.  The installer uses `setxkbmap` and `xkbcomp` and needs a running X
display.

## Configuration and references

The user-facing configuration is
[`shirolyzer/config/config.yaml`](shirolyzer/config/config.yaml).  It controls
the physical layout, current layout, corpora and weights, primary-sacrifice
budget, fingermap, alt-finger rules, geometric conditions, objective priority
tiers, and finger balance.  `general.current_layout` is the canonical layout
used by `analyze.py` when `--layout` is omitted and by `compute.py` as its
punctuation/dead-key template; search seeds come from the reference file.

Reference layouts live in
[`shirolyzer/config/references.toml`](shirolyzer/config/references.toml).
`compute.py` appends newly validated layouts to that file (or to
`--append-references`) so future runs can seed from them.

## Repository map

```text
corpora/shai/             English corpus parser and parsed frequencies
corpora/spanish/          Spanish corpus parser and parsed frequencies
shirolyzer/               Primary entry points and runtime package
shirolyzer/commands/      Implementations of the CLI subcommands
shirolyzer/config/        YAML config, references, parameters, score baseline
shirolyzer/tests/         Unit and regression tests
shirolyzer/results/       Local outputs and committed result reports
shirolyzer/fitting/       Model-refitting development tools
xkb/                      Generated symbols and installer
research/                 Historical investigation reports
```

## Documentation

- [`shirolyzer/README.md`](shirolyzer/README.md) — full usage, configuration,
  corpora, search phases, diversity output, tests, and XKB workflow.
- [`shirolyzer/results/rust/PARITY_REPORT.md`](shirolyzer/results/rust/PARITY_REPORT.md)
  — Python-vs-Rust parity results.
- [`shirolyzer/results/rust/BENCHMARKS.md`](shirolyzer/results/rust/BENCHMARKS.md)
  — CPU optimization and parallel-seed measurements.
- [`shirolyzer/results/rust/GPU_FEASIBILITY.md`](shirolyzer/results/rust/GPU_FEASIBILITY.md)
  — GPU probe, decision, and `--device` fallback.
- [`shirolyzer/results/rust/TUNING_REPORT.md`](shirolyzer/results/rust/TUNING_REPORT.md)
  — parameter sweeps and final defaults.
- [`shirolyzer/results/rust/DIVERSITY_REPORT.md`](shirolyzer/results/rust/DIVERSITY_REPORT.md)
  — near-equal distributions found during tuning.
- [`shirolyzer/results/rust/SUMMARY.md`](shirolyzer/results/rust/SUMMARY.md)
  — final completion record and gate results.
- [`shirolyzer/results/calibration/REPORT.md`](shirolyzer/results/calibration/REPORT.md)
  — runtime calibration and default rationale.
- [`shirolyzer/results/validation/REPORT.md`](shirolyzer/results/validation/REPORT.md)
  — post-rewrite validation and reference comparison.
- [`shirolyzer/results/validation/T5-REPORT.md`](shirolyzer/results/validation/T5-REPORT.md)
  — diversity-aware validation example.
- [`shirolyzer/results/round8/REPORT_geo.md`](shirolyzer/results/round8/REPORT_geo.md)
  — historical round-8 pattern-model report, with its audit and fingering review.

## Tests

```bash
cd rust
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --release
cargo run --release -- check-baseline
```

The frozen Python oracle is still runnable with
`python3 -m unittest discover -s shirolyzer/tests -v` and
`python3 -m shirolyzer check-baseline`.

---

Built by [akai_hana](https://github.com/akai-hana).
