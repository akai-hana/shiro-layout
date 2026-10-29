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

The analyzer, generator, auditor, and comparer live directly under
`shirolyzer/`.  Archived pre-rewrite code remains under `shirolyzer/legacy/`
for reference and is not used at runtime.

## Quick start

Requires Python 3.11+ and the dependencies in `requirements.txt`.

```bash
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt
```

Build the parsed corpora (network access is required):

```bash
python3 corpora/shai/parse_shai.py
python3 corpora/spanish/parse_spanish.py
```

Analyze the current default layout:

```bash
python3 shirolyzer/analyze.py \
  --layout ";tspbxgoiwndhrlyecuafkvmzjq,./" \
  --primary-corpus corpora/shai/parsed \
  --secondary-corpus corpora/spanish/parsed
```

Generate a layout and write it to XKB:

```bash
python3 shirolyzer/compute.py --name shiro --xkb --xkb-name shiro
```

A default compute run is a long-running search; see
[`shirolyzer/README.md`](shirolyzer/README.md) for shorter experiments and for
the meaning of `selected` versus the full `valid_layouts` list.

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
the physical layout, default layout, corpora and weights, primary-sacrifice
budget, fingermap, alt-finger rules, geometric conditions, objective priority
tiers, and finger balance.

Reference layouts live in
[`shirolyzer/config/references.toml`](shirolyzer/config/references.toml).
`compute.py` appends newly validated layouts to that file (or to
`--append-references`) so future runs can seed from them.

## Repository map

```text
corpora/shai/             English corpus parser and parsed frequencies
corpora/spanish/          Spanish corpus parser and parsed frequencies
shirolyzer/               Analyzer/generator entry points and runtime package
shirolyzer/config/        YAML config, references, theta, score baseline
shirolyzer/tests/         Unit and regression tests
shirolyzer/results/       Local outputs and committed calibration reports
shirolyzer/dev/           Fitting and baseline development tools
shirolyzer/legacy/        Archived pre-rewrite code
xkb/                      Generated symbols and installer
research/                 Historical investigation reports
```

## Documentation

- [`shirolyzer/README.md`](shirolyzer/README.md) — full usage, configuration,
  corpora, search phases, diversity output, tests, and XKB workflow.
- [`shirolyzer/results/calibration/REPORT.md`](shirolyzer/results/calibration/REPORT.md)
  — runtime calibration and default rationale.
- [`shirolyzer/results/validation/REPORT.md`](shirolyzer/results/validation/REPORT.md)
  — post-rewrite validation and reference comparison.
- [`shirolyzer/results/validation/T5-REPORT.md`](shirolyzer/results/validation/T5-REPORT.md)
  — diversity-aware validation example.

## Tests

```bash
python3 -m compileall shirolyzer
python3 shirolyzer/check_baseline.py
python3 -m unittest discover -s shirolyzer/tests -v
```

---

Built by [akai_hana](https://github.com/akai-hana).
