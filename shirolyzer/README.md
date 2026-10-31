# shirolyzer【白ライザー】

Layout analyzer and generator built around the calibrated, context-dependent
pattern model used by the Shiro layout.  It can score an existing layout,
audit the model's fingering rules, rank reference layouts, and search for new
30-key arrangements under primary and secondary corpora.

The runtime is the Rust binary built from this directory.  It exposes seven
subcommands (`compute`, `analyze`, `render`, `xkb`, `compare`, `audit`,
`check-baseline`) and is the target of the defaults and benchmarks below.
Build it with:

```bash
cargo build --manifest-path shirolyzer/Cargo.toml --release
```

The Python modules under `shirolyzer/utils/` and `shirolyzer/fitting/` are
development tooling only: the corpus parsers and the model-refitting tools.
They are not part of the runtime and are not installed with the binary.

All commands below run from the repository root.  Replace
`shirolyzer/target/release/shirolyzer` with `just <recipe>` if you use the
provided `justfile`.

## Requirements

- Rust 1.99 or newer for the analyzer.
- Python 3.11 or newer with `numpy`, `scipy`, and `PyYAML` only when using the
  corpus parsers or refitting tools.

```bash
python3 -m venv .venv
. .venv/bin/activate
pip install numpy scipy PyYAML
```

## Quick start

Build the parsed corpora, analyze the current layout, and generate an XKB
symbols file:

```bash
cargo build --manifest-path shirolyzer/Cargo.toml --release

python3 corpora/shai/parse_shai.py
python3 corpora/douglas/parse_douglas.py

shirolyzer/target/release/shirolyzer analyze ";ntgfjoehkrpmscuaydilbqvwxz,./" --name shiro
shirolyzer/target/release/shirolyzer compute --name shiro --xkb --xkb-name shiro
xkb/install.sh shiro
```

Render any layout as a staggered ASCII grid; by default each key is shown with
its configured finger, like the `fingermap` in `config.yaml`:

```bash
shirolyzer/target/release/shirolyzer render ";ntgfjoehkrpmscuaydilbqvwxz,./"
shirolyzer/target/release/shirolyzer render --plain ";ntgfjoehkrpmscuaydilbqvwxz,./"
shirolyzer/target/release/shirolyzer render --both ";ntgfjoehkrpmscuaydilbqvwxz,./"
```

Common workflows are also available through `just`:

```bash
just compute
just analyze ";ntgfjoehkrpmscuaydilbqvwxz,./"
just render ";ntgfjoehkrpmscuaydilbqvwxz,./"
just test
just check
```

`compute` with defaults is a long-running search.  See
[Search and diversity](#search-and-diversity) for shorter experiments and
for the way valid layouts are reported.

## Repository layout

| Path | Purpose |
|---|---|
| `shirolyzer/` | Rust crate root (manifest, binary, library) |
| `shirolyzer/src/` | CLI, model, scorer, search, and output implementations |
| `shirolyzer/tests/` | Rust unit, parity, and command tests |
| `shirolyzer/benches/` | Criterion benchmarks |
| `shirolyzer/config/` | YAML config, references, model parameters, score baseline |
| `shirolyzer/results/` | Local run outputs plus committed result reports |
| `shirolyzer/fitting/` | Model-refitting development tools |
| `shirolyzer/utils/` | Python support modules for the development tools |
| `corpora/shai/` | Shai English corpus parser and parsed frequency files |
| `corpora/douglas/` | Spanish corpus parser and parsed frequency files |
| `xkb/` | Generated symbols and the installer |

`shirolyzer/results/` is gitignored except for explicitly added reports and
validation/round-8 artifacts.

## Configuration

`shirolyzer/config/config.yaml` is the user-facing source of truth for the
physical layout, corpora, fingermap, alt-finger rules, geometric conditions,
objective priorities, finger balance, and optional per-hand character
restrictions.  Fitted model parameters live in the parameter file referenced
by the config.

The shipped `model/pattern_parameters_A_all.json` is a JSON object mapping the
named coefficients in `utils/comfort.py` to their calibrated values.  `A` is the
model variant (full-span middle/ring reverse scissors are disallowed) and `all`
is the alt-finger gate set.  The fitting tools start from `fit_seed_A.json` and
write candidate parameter files under `shirolyzer/results/fitting/`; they are
not read unless you explicitly point a config at them.

An abridged view of the main sections:

```yaml
general:
  physical_layout: ansi
  current_layout: ";ntgfjoehkrpmscuaydilbqvwxz,./"
  primary_corpus:
    - {name: shai, weight: 100}
  secondary_corpus:
    - {name: douglas, weight: 100}
  primary_sacrifice: 0.00023   # percent of the best primary score

model:
  type: pattern
  variant: A
  parameters_file: model/pattern_parameters_A_all.json
```

`general.current_layout` is the canonical layout for this repository.
`analyze` uses it when no layout argument is given, and `compute` uses it only
to choose punctuation/dead-key positions.  Search seeds come from
`references.toml`, not from this setting.

The remaining sections control the search:

- `fingermap`: 30 space-separated key/finger tokens in QWERTY row-major order.
  A token is a QWERTY key label followed by `P`, `R`, `M`, or `I` (pinky,
  ring, middle, index), for example `QR` or `;P`.
- `hand_restrictions`: optional `left` and `right` lists (or strings) of
  letters.  `compute` keeps each listed letter on keys of that hand; unlisted
  letters are free.  Seeds that violate a restriction are skipped with a
  warning.
- `alt_fingers`: context gates and explicit sequence overrides.  A gate uses
  `key`, `finger`, `mode` (`only`, `block`, or `never`), and an optional
  `context` list of QWERTY keys.  An explicit override uses `sequence` plus a
  `fingers` list with one finger per key.  Set `explicit: true` on a gate when
  it must be enforced through the explicit context table.
- `conditions`: ordered rules for `scissors`, `spread`, `separation`, and
  `crossing`.  The first matching rule wins; `allow: false` rejects the
  fingering and `allow: true` permits it.
- `priority_list`: tiers of objective terms.  Higher tiers receive larger
  multipliers, and terms in one list share a tier.  Every supported name
  (`sfbs-sfts`, `scissors`, `redirects`, `rolls`, `alternation`, and
  `finger_balance`) must appear exactly once.
- `finger_balance`: target percentage for each hand/finger.  Values are
  normalised, so they do not have to add up to 100.

Corpus entries may use a `name` that resolves to
`corpora/<name>/parsed`, or an explicit `path` to a parsed directory, a corpus
directory containing `parsed/`, or a raw `.txt`, `.xz`, or `.zip` source.
Weights blend corpora by scaling their counts.

Command-line corpus options replace the YAML corpus lists for that run:
`--primary-corpus` and `--secondary-corpus` are repeatable and merge their
inputs by count.  `--config PATH` loads a different YAML file; relative
parameter paths are resolved from `shirolyzer/config/`.

Validation rejects unknown sections, malformed tokens, unknown priority names,
duplicate priorities, bad conditions, and missing model parameters with an
error that names the config location.

### Reference layouts

`shirolyzer/config/references.toml` is an array of tables:

```toml
[[reference]]
name = "shiro"
layout = ";ntgfjoehkrpmscuaydilbqvwxz,./"
primary_score = 3.520201897
secondary_score = 3.704852567
seed = 20261029
```

Only `name` and `layout` are required.  A layout must have 30 keys with 26
distinct letters, and names must be unique.  Scores and seed are informational.
`compute` appends each valid layout to the reference file (or to
`--append-references`) with a unique name; a layout that is already present is
not appended again.  Use `compare --refs PATH` to rank a different file.

## Corpora

Both parsers are development/build-time tools.  They download their documented
source, parse it, write the three frequency JSON files under `parsed/`, and
delete the temporary source on success.  Network access is required for the
default mode.

### Shai English

```bash
python3 corpora/shai/parse_shai.py
python3 corpora/shai/parse_shai.py --check
python3 corpora/shai/parse_shai.py --source FILE.txt.xz --keep-source
```

Output: `corpora/shai/parsed/{letter,bigram,trigram}_freq.json`.  The parser
lowercases and counts maximal `[a-z]+` runs; non-letters break runs.  Entries
are ordered from most to least frequent, and the parser reports whether it
changed any files.  See `corpora/shai/README.md` for source details and totals.

### Douglas (Spanish)

```bash
python3 corpora/douglas/parse_douglas.py
python3 corpora/douglas/parse_douglas.py --check
python3 corpora/douglas/parse_douglas.py --source FILE.zip --keep-source
```

Output: `corpora/douglas/parsed/{letter,bigram,trigram}_freq.json`.  The parser
normalises with NFKD, strips combining marks, lowercases, and counts maximal
`[a-z]+` runs.  Entries are ordered from most to least frequent, and the parser
reports whether it changed any files.  See `corpora/douglas/README.md` for
source details and totals.

`--check` parses and compares against the existing `parsed/` files without
writing.  `--source` uses a local archive instead of downloading.  `--url`
overrides the download URL.  `--keep-source` keeps a downloaded temporary
source for debugging.

## Analysis

### Analyze one layout

```bash
shirolyzer/target/release/shirolyzer analyze ";ntgfjoehkrpmscuaydilbqvwxz,./" --name shiro
```

With no layout argument, the config's `general.current_layout` is used.  With
no corpus flags, the YAML corpora are used.  `--layout-flag` remains available
as a compatibility way to pass the layout.

The report contains the primary and secondary corpus scores, bigram category
shares, the worst bigrams, scissor counts, trigram patterns, finger load, and a
per-letter fingering guide.  Output defaults to
`shirolyzer/results/analysis[_name].txt` and `.json`; override with `--out` and
`--out-json`.

### Audit the model

```bash
shirolyzer/target/release/shirolyzer audit \
  --out shirolyzer/results/audit.txt \
  --out-json shirolyzer/results/audit.json
```

The audit prints questionnaire probes, hard checks, corpus totals, and every
model ruling with a pass/fail result.  The default YAML model is used unless a
legacy model configuration name such as `A:all` is supplied positionally.
`--rulings PATH` loads a custom rulings list.  `--config PATH` selects a
different YAML file.

### Compare references

```bash
shirolyzer/target/release/shirolyzer compare
shirolyzer/target/release/shirolyzer compare --refs shirolyzer/config/references.toml
shirolyzer/target/release/shirolyzer compare --primary-corpus corpora/shai/parsed
```

`compare` prints reference names, plain corpus trigram scores, and layouts,
sorted by score.  This is the plain trigram cost used by `analyze`; the
generator's primary score also includes the calibrated comfort, geometry,
condition, and balance terms, so the two numbers are not directly comparable.

### Regression check

```bash
shirolyzer/target/release/shirolyzer check-baseline
```

This checks reference bigram/trigram costs against
`shirolyzer/config/score_baseline.json`, the audit ruling count, and the
shipped-model baseline.  Newly appended references are allowed; references
listed in the baseline must still match within `1e-9`.  Use `--write` only when
intentionally regenerating the baseline.

## Search and diversity

`compute` combines the reference layouts with random restarts, optimizes them
on the primary corpus, then tunes promising layouts on the secondary corpus
without exceeding the primary-sacrifice budget.

```bash
shirolyzer/target/release/shirolyzer compute --name run --seed 20261029
```

Defaults:

| Option | Default | Meaning |
|---|---:|---|
| `--runs` | `28` | Random restarts in addition to reference seeds |
| `--iters` | `200000` | Anneal iterations per primary run |
| `--ils` | `8` | ILS rounds per top primary layout |
| `--primary-sacrifice` | `0.00023` | Percent of best primary allowed for secondary tuning |
| `--valid-tolerance` | `0.002` | Primary window for valid layouts |
| `--secondary-tolerance` | `0.005` | Secondary window for valid layouts |
| `--min-layout-distance` | `4` | Minimum key-position distance between valid layouts |
| `--max-valid` | `8` | Maximum valid layouts reported; `0` means no limit |
| `--polish-rounds` | `0` | Maximum polish passes over distinct candidates; `0` disables |
| `--polish-candidates` | `4` | Distinct primary candidates to polish |

The Rust effort defaults are validated in
`shirolyzer/results/rust/TUNING_REPORT.md`: the search reaches the same optimum
at 25k iterations, four restarts, and two ILS rounds, so the shipped
`200k/28/8` values sit well past the plateau and remain robust for other
corpora.  The older Python calibration is in
`shirolyzer/results/calibration/REPORT.md`.  Add `--device gpu` to request a
GPU run; this build warns and falls back to CPU
(`shirolyzer/results/rust/GPU_FEASIBILITY.md`).  Override any value per run.
`--time-budget` accepts seconds or strings such as `30s`, `10m`, or `1h`, stops
between runs/ILS/secondary operations, and always saves the best result found
so far.

### Search phases

1. **Primary seeding and search.** Load reference layouts, add explicit
   `--seed-layout` layouts, then add `--runs` random restarts.  Every seed is
   quenched, annealed for `--iters`, and quenched again on the primary corpus.
2. **ILS.** The four best distinct primary layouts receive up to `--ils`
   rounds of perturbation, quench, and half-length anneal.  Improvements are
   kept between rounds.
3. **Optional polish.** With `--polish-rounds N`, the top distinct candidates
   within the valid-layout tolerances are polished for at most `N` passes
   each.  A candidate stops early when a full pass does not improve its primary
   score.
4. **Secondary phase.** The four best primary layouts are tuned against the
   secondary corpus under a hard primary limit.  The limit is the best primary
   score plus `--primary-sacrifice` percent.  The best eligible secondary result
   becomes `selected`.
5. **Diversity validation.** Results are merged by layout, clustered by score
   and key distance, and reported as `valid_layouts`.

### Valid layouts

A result is valid when its primary score is no more than `--valid-tolerance`
worse than the best primary, and, when both have a secondary score, its
secondary score is no more than `--secondary-tolerance` worse.  Only layouts at
least `--min-layout-distance` key positions apart are kept, up to
`--max-valid`.  A row that is better on either score is never disqualified by
that score.

If one valid layout remains, `selection_status` is `clear_winner`.  Otherwise
it is `multiple_valid`: one layout is still selected for XKB and summary
output, but every valid layout is printed and written to the JSON
`valid_layouts` list, and each is appended to the reference file with a unique
name.  The runner-up may have a better secondary score while giving up some
primary score; inspect the valid list rather than treating `selected` as the
only result.  See `shirolyzer/results/validation/T5-REPORT.md` for an example
with two valid layouts.

### Examples

A short experiment that does not modify the real reference file:

```bash
cp shirolyzer/config/references.toml /tmp/shiro-refs.toml
shirolyzer/target/release/shirolyzer compute \
  --references /tmp/shiro-refs.toml \
  --append-references /tmp/shiro-refs.toml \
  --time-budget 2m --runs 2 --iters 20000 --ils 1 \
  --name try
```

A diversity-aware search with polishing:

```bash
shirolyzer/target/release/shirolyzer compute --name diversify --seed 20261030 \
  --polish-rounds 3 --polish-candidates 6 \
  --valid-tolerance 0.002 --secondary-tolerance 0.005 \
  --min-layout-distance 4 --max-valid 8
```

A full default run that writes the selected layout to XKB:

```bash
shirolyzer/target/release/shirolyzer compute --name shiro --seed 20261029 \
  --xkb --xkb-name shiro
```

`--seed` makes a run reproducible; when omitted, a seed is generated and
recorded in the output.  `--seed-layout LAYOUT` adds a starting layout and is
repeatable.

### Output

Default output paths are
`shirolyzer/results/compute[_name].txt` and `.json`.  The text output records
the seed, corpora, references, selection status, best primary/secondary, the
valid-layout block, and every result row.  The JSON also includes elapsed time,
the time budget, completed run/iteration/ILS/secondary/polish counts, all
parameters, `results`, `best_primary`, `best_secondary`, `selected`,
`selection_status`, `valid_layouts`, appended reference names, and the XKB path
when requested.

`compute` appends valid layouts by default because the reference list is part
of the calibration workflow.  For experiments, point both `--references` and
`--append-references` at a temporary copy.

## XKB workflow

Generate symbols for an existing layout:

```bash
shirolyzer/target/release/shirolyzer xkb \
  --layout ";ntgfjoehkrpmscuaydilbqvwxz,./" \
  --name shiro
```

This writes `xkb/shiro`.  Use `--out PATH` for a different location, or
`--install` to run the installer immediately.

Generate from a compute run:

```bash
shirolyzer/target/release/shirolyzer compute --xkb --xkb-name shiro --name run
```

Without `--xkb-name`, the selected layout is written under a generated safe
name.  `xkb/mine` is only overwritten when `--xkb-name mine` is passed
explicitly.

Install and apply a symbols file to the current X display:

```bash
xkb/install.sh                 # install xkb/mine as "mine"
xkb/install.sh shiro           # install xkb/shiro as "shiro"
xkb/install.sh FILE NAME       # install an arbitrary symbols file
```

The installer needs a running X display.  It uses `setxkbmap` to print the
component shell and `xkbcomp` to compile the final keymap, because `setxkbmap
-I` replaces the system XKB path.  Generated symbols use the same punctuation
mapping as `xkb/mine`, keep the apostrophe on both `<AC11>` and the ISO
`<LSGT>` key, and can be compiled with `xkbcomp` for a dry-run check.

## Tests and quality gates

```bash
cargo fmt --manifest-path shirolyzer/Cargo.toml --check
cargo clippy --manifest-path shirolyzer/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path shirolyzer/Cargo.toml --release
shirolyzer/target/release/shirolyzer check-baseline
shirolyzer/target/release/shirolyzer audit
```

The Rust tests cover config validation, reference baselines, table shapes,
delta scoring, diversity selection, and the command outputs.  The retained
development tools can be checked with:

```bash
python3 -m compileall -q shirolyzer corpora
python3 -c "import shirolyzer.fitting, shirolyzer.utils.fitlib, shirolyzer.utils.gateset"
```

## Development tools

The Python code is not the runtime.  `corpora/*/parse_*.py` build the parsed
corpus files, and `shirolyzer/fitting/` refits the calibrated pattern model.
They share the Python support modules in `shirolyzer/utils/` (`comfort`,
`config`, `conditions`, `fitlib`, `gates`, `gateset`, `rulings`, `paths`,
`fingers`, `ansi`).  Keep the Rust binary and these tools in sync when you
change the model or corpus formats.  See `shirolyzer/fitting/README.md` for the
fitting commands.

## Calibration and validation

- `shirolyzer/results/calibration/REPORT.md` — runtime sweeps and the rationale
  for the current run/iteration/ILS defaults.
- `shirolyzer/results/validation/REPORT.md` — post-rewrite full validation,
  audit, and reference comparison.
- `shirolyzer/results/validation/T5-REPORT.md` — diversity-aware validation
  with two materially different valid layouts.
- `shirolyzer/results/round8/REPORT_geo.md` — historical round-8 pattern-model
  report, with its audit output, leave-one-out log, and fingering review.
- `shirolyzer/results/rust/TAKEOVER_SUMMARY.md` — takeover completion record,
  publication status, and Python-removal decision.
- `shirolyzer/results/rust/TAKEOVER_VERIFICATION.md` — post-takeover gates and
  benchmark comparison.

Committed reports and artifacts remain in the repository for traceability;
other `shirolyzer/results/` files are local outputs.

---

Built by [akai_hana](https://github.com/akai-hana).
