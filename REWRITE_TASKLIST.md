# Shirolyzer Rewrite — Tasklist and Implementation Plan

Source specification: `REWRITE_SPEC_LIST.txt` (must remain unchanged).
Status: planning complete; implementation not started.
Last planned: 2026-10-28.

This file is the implementation plan and progress tracker for the rewrite. It also records the
decisions made during planning so the implementation does not have to rediscover them.

---

## 1. Locked decisions

The following decisions were confirmed during planning:

1. **Configuration format:** YAML.
   - Default file: `shirolyzer/config/config.yaml`.
   - The YAML may contain comments and nested lists/tables.
   - Alt-finger sequences, conditions, and priority tiers use the structured YAML forms rather
     than the non-YAML syntax shown in the original spec examples.
2. **Analyzer layout:** no `core/` sub-directory.
   - `analyze.py`, `compute.py`, and `audit.py` live directly under `shirolyzer/`.
   - `compare.py` also remains a user-facing entry point directly under `shirolyzer/`.
   - Shared code lives under `shirolyzer/utils/`; user configuration lives under
     `shirolyzer/config/`.
   - The branch name `core` is still used for the analyzer-architecture batch because the spec
     names it that way; it no longer refers to a directory.
3. **Priority semantics:** tiered weighted sum.
   - Items in the same YAML list item (for example `[scissors, redirects]`) have equal weight.
   - Higher tiers receive a larger multiplier than lower tiers.
   - The default configuration must preserve the currently calibrated scoring behaviour.
4. **`alts` in the priority list means hand alternation**, not alternate fingering.
   - The priority field is renamed to `alternation`.
   - Alternate fingering remains under `alt_fingers`.
5. **Secondary corpus phase:** constrained local search after primary convergence.
   - The main optimization loop evaluates only the primary corpus.
   - Secondary-corpus improvement is attempted only after primary convergence, and candidates
     outside the configured primary-sacrifice budget are rejected.
6. **Behaviour baseline:** preserve the current calibrated model by default.
   - Current weights, implicit alt-context rules, and reference layouts are ported into the
     default configuration.
   - A regression check must confirm that reference scores still match the pre-rewrite values.
7. **Runtime tuning:** time-budget-driven experimentation.
   - `compute.py` gains a `--time-budget` flag.
   - Runs under several time budgets will be compared to find where increasing iterations,
     runs, and ILS rounds stops helping.
   - New defaults will be chosen from that data.
8. **Reference list:** `shirolyzer/config/references.toml`.
   - `compare.py` reads references from this file instead of hard-coded `REF` data.
   - `compute.py` appends its final layout to this file when a run finishes.

---

## 2. Assumptions and conventions

These are working assumptions unless a question below says otherwise:

- The default YAML configuration is the only required user config file.
- Downloaded corpus source archives are temporary and are never committed.
- Only the parsed frequency JSON files, corpus READMEs, and configuration/reference files remain
  in version control.
- `shirolyzer/results/` and `shirolyzer/legacy/` remain in place. `legacy/` is frozen: active code
  must not import it, but it does not have to be migrated to the new package layout.
- The ANSI geometry becomes `shirolyzer/utils/ansi/` with an `ansi.py` inside it and an
  `__init__.py` re-exporting the public geometry functions. Future `iso/` and `matrix/` packages
  can follow the same pattern.
- Primary entry points are run directly, for example `python shirolyzer/compute.py ...`.
  Because the scripts live in `shirolyzer/`, imports use `utils.*` as the import root. No
  hard-coded absolute paths or `sys.path.insert(...)` calls are allowed.
- Reference TOML entries use an array of tables (`[[reference]]`) so `compute.py` can append new
  entries without rewriting existing comments.
- `primary_sacrifice` in YAML is a percentage of the best primary score. The default value must be
  chosen to match the current near-zero sacrifice behaviour.

---

## 3. Branch, issue, and commit workflow

Branches are created in the order below, from an up-to-date `main`. Never commit directly to
`main`, with one approved exception: the initial `REWRITE_TASKLIST.md` commit is made directly to
`main`.

| Order | Branch | Milestone scope | Exit condition |
|---|---|---|---|
| 0 | `main` (one-time direct commit) | Initial tasklist | Plan committed to `main` |
| 1 | `corpus` | Corpus scripts, download/parse pipelines, parsed data layout | All corpus tasks pass checks and merge |
| 2 | `core` | Package layout, entry points, imports, paths, two-phase search | Analyzer works corpus-independently and regression scores match |
| 3 | `config` | YAML schema, fingermap, alt fingers, conditions, priorities | Default config reproduces current behaviour |
| 4 | `xkb` | XKB generation and installer workflow | Generated symbols match existing symbols for the same layout |
| 5 | `calibration` | Time-budget flag, tuning experiments, new defaults | New defaults justified by recorded results |
| 6 | `docs` | READMEs, usage docs, final stale-reference cleanup | Final checks pass and docs match the implementation |

Rules:

- One task below equals one commit. Use the commit message written on the task.
- Push every commit to the current task branch.
- At the end of a branch, merge it into `main` only after all its tasks are checked and the branch
  verification commands pass.
- Create the next branch from the updated `main`.
- If the plan changes, edit this file and the corresponding GitHub issues before continuing with
  implementation.
- Never modify `REWRITE_SPEC_LIST.txt`.

### GitHub issue setup

- Create one issue per task below.
- Issue title: the task title.
- Labels: `rewrite` plus the branch name (`corpus`, `core`, `config`, `xkb`, `calibration`,
  `docs`).
- Milestone: the branch name or the phase name.
- Fill the `Issue` field in each task with the issue number after creation.
- The tracking comments in this file are the source of truth for progress.

---

## 4. Quality gates

Run these checks at the end of every branch, in addition to the task-specific checks:

```bash
python -m compileall shirolyzer
python shirolyzer/analyze.py --help
python shirolyzer/compute.py --help
python shirolyzer/audit.py --help
python shirolyzer/compare.py --help
git grep -nE "analyze_shai|anneal_shai|audit_shai|build_shai_corpus|build_spanish_corpus|scorer2|search2|/home/akai|sys\.path\.insert"
```

Acceptable remaining matches are only in `shirolyzer/legacy/`, `shirolyzer/results/`, historical
documentation, or this tasklist itself. Any other match must be fixed.

---

## 5. Tasks

### Phase 0 — Planning and tracking

- [x] **P0.1 — Commit the plan directly to `main` (approved one-time exception)**
  - Issue: none
  - Commit: `Plan tasklist`
  - This is the only direct commit to `main`; all later work happens on task branches.
  - Done when `REWRITE_TASKLIST.md` is on `origin/main`.

- [ ] **P0.2 — Publish the plan as GitHub issues and link them**
  - Issue: TBD
  - Commit: `planning: link rewrite issues`
  - Create the issues described in section 3, fill in every `Issue: TBD` field, and commit this
    tracking update on the `corpus` branch before starting C1.
  - Done when every task has an issue number and no implementation task is unassigned.

- [ ] **P0.3 — Create the `corpus` branch from `main`**
  - Issue: TBD
  - Commit: none (branch creation only)
  - Done when `corpus` exists locally and remotely, created from the latest `main`.

### Branch `corpus`

- [ ] **C1 — Move and rename the corpus parser scripts**
  - Issue: TBD
  - Commit: `corpus: move and rename corpus parsers`
  - Files: `shirolyzer/build_shai_corpus.py` → `corpora/shai/parse_shai.py`;
    `shirolyzer/build_spanish_corpus.py` → `corpora/spanish/parse_spanish.py`.
  - Update shebangs, docstrings, usage examples, and path resolution for the new locations.
  - Do not add download logic yet; keep this commit limited to the move and rename.
  - Done when both scripts run from their new directories and no active file references the old
    names.

- [ ] **C2 — Standardise the `parsed/` output directories**
  - Issue: TBD
  - Commit: `corpus: standardize parsed corpus directories`
  - Ensure `corpora/shai/parsed/` and `corpora/spanish/parsed/` both exist and contain only
    `letter_freq.json`, `bigram_freq.json`, and `trigram_freq.json`.
  - Move the tracked Spanish JSON files from `corpora/spanish/` into
    `corpora/spanish/parsed/`.
  - Keep JSON key ordering and formatting consistent between both corpora.
  - Done when both parsers write to `parsed/` and the old Spanish root-level JSON files are gone.

- [ ] **C3 — Make `parse_shai.py` self-contained**
  - Issue: TBD
  - Commit: `corpus: make shai parser self-contained`
  - Add download and streaming decompression for the `.xz` source named in
    `corpora/shai/README.md` (standard library only: `urllib`, `lzma`, `hashlib`, `tempfile`).
  - Support `--source PATH` for an existing local source, `--url URL` for an override, `--check`
    for comparison only, `--write` to write JSON, and `--keep-source` for debugging.
  - Default behaviour downloads to a temporary location, parses, writes the three JSON files to
    `corpora/shai/parsed/`, and deletes the downloaded source on success.
  - Verify the generated counts against the current parsed data, including the documented totals:
    418,381,828 letters, 327,253,737 bigrams, and 240,524,895 trigrams.
  - Done when a fresh run produces the same JSON content as the tracked parsed files.

- [ ] **C4 — Make `parse_spanish.py` self-contained**
  - Issue: TBD
  - Commit: `corpus: make spanish parser self-contained`
  - Replace the old local-directory assumption with the Zenodo zip source named in
    `corpora/spanish/README.md`.
  - Download the zip, extract only the needed text files to a temporary directory, normalise with
    the existing NFKD/strip-combining-marks rules, parse, write the three JSON files to
    `corpora/spanish/parsed/`, and delete the zip and extracted files on success.
  - Support the same `--source`, `--url`, `--check`, `--write`, and `--keep-source` options as C3.
  - Done when a fresh run produces valid parsed JSON and leaves no source archive behind.

- [ ] **C5 — Add corpus and Python cache ignores**
  - Issue: TBD
  - Commit: `corpus: ignore downloaded corpus sources and caches`
  - Extend `.gitignore` for downloaded corpus archives, decompressed/extracted raw sources,
    `__pycache__/`, `*.py[cod]`, and temporary corpus directories.
  - Do not ignore the tracked `parsed/*.json` files.
  - Done when downloading/parsing a corpus leaves a clean `git status` apart from intentional JSON
    changes.

- [ ] **C6 — Update corpus documentation and remove solved TODOs**
  - Issue: TBD
  - Commit: `docs: update corpus readmes`
  - Document the exact commands for both parsers, the source URLs, the output paths, and the fact
    that raw sources are deleted.
  - Correct the Spanish counts currently copied from the Shai README with the real parsed totals.
  - Remove `corpora/spanish/TODO.txt` or mark it resolved, since C4 replaces the old corpus source.
  - Done when the corpus READMEs match the scripts and generated data.

Branch exit: `corpus` merges to `main`; create `core` from the updated `main`.

### Branch `core`

- [ ] **K1 — Establish the import root and package skeleton**
  - Issue: TBD
  - Commit: `core: add package skeleton and dependencies`
  - Create `shirolyzer/utils/__init__.py`, `shirolyzer/utils/ansi/__init__.py`, and the
    `shirolyzer/config/` directory. `analyze.py`, `compute.py`, `audit.py`, and `compare.py` stay
    directly under `shirolyzer/`.
  - Add `requirements.txt` with `numpy` and `PyYAML` if they are not already declared elsewhere.
  - Keep direct execution (`python shirolyzer/analyze.py`) as the primary interface, with
    `shirolyzer/` as the import root (`from utils...`). Do not add wrapper scripts.
  - Done when every script can import its local modules without an absolute `sys.path` insert.

- [ ] **K2 — Rename the analyzer entry points**
  - Issue: TBD
  - Commit: `core: rename analyzer entry points`
  - Rename `analyze_shai.py` → `analyze.py`, `anneal_shai.py` → `compute.py`, and
    `audit_shai.py` → `audit.py`.
  - Remove `_shai` from docstrings, help text, comments, and default values where it implies a
    corpus-specific tool.
  - Keep `compare.py` as a user-facing entry point.
  - Done when the three renamed scripts run and the old filenames no longer exist.

- [ ] **K3 — Move shared modules under `utils/`**
  - Issue: TBD
  - Commit: `core: move shared analysis modules under utils`
  - Move `comfort.py`, `fingers.py`, `ils.py`, and `objective.py` into `shirolyzer/utils/`.
  - Update all imports in the active scripts to import from `utils.*`.
  - Leave `legacy/` untouched; it is intentionally frozen.
  - Done when all active scripts import the moved modules and no active file imports top-level
    `comfort`, `fingers`, `ils`, or `objective`.

- [ ] **K4 — Consolidate the scorer and search modules**
  - Issue: TBD
  - Commit: `core: consolidate scorer and search modules`
  - Rename `scorer2.py` → `shirolyzer/utils/scorer.py` and `search2.py` →
    `shirolyzer/utils/search.py`, dropping the `2` suffix.
  - Fix the stale `objective.py` dependency on `scorer.Corpus` and keep the `Corpus` interface
    available from the new scorer module.
  - Update every active import of `scorer2`/`search2`.
  - Done when active code contains no `scorer2` or `search2` references.

- [ ] **K5 — Package the ANSI geometry by physical layout**
  - Issue: TBD
  - Commit: `core: package ansi geometry under utils/ansi`
  - Move `ansi.py` to `shirolyzer/utils/ansi/ansi.py`.
  - Add `shirolyzer/utils/ansi/__init__.py` that re-exports the public geometry API and provides
    a registry entry for `physical_layout = "ansi"`.
  - Structure the registry so `iso`, `matrix`, and similar one-word physical-layout packages can
    be added later without changing callers.
  - Update imports in `comfort.py`, `scorer.py`, and `audit.py`.
  - Done when `physical_layout: ansi` resolves through the registry and all geometry imports work.

- [ ] **K6 — Remove absolute paths and result-path hard-coding**
  - Issue: TBD
  - Commit: `core: replace absolute paths with repo-relative resolution`
  - Delete all `/home/akai/...` paths and `sys.path.insert(...)` calls from active code.
  - Add a small path helper under `utils/` for repository root, corpus root, config directory, and
    results directory.
  - Replace hard-coded `results/round8/...` output paths with `--out`/`--out-json` options whose
    defaults are derived from the command and layout name.
  - Keep existing `results/` history untouched.
  - Done when `git grep` finds no absolute paths in active code and all commands have explicit
    output options.

- [ ] **K7 — Remove corpus-specific assumptions from the entry points**
  - Issue: TBD
  - Commit: `core: make entry points corpus-independent`
  - `analyze.py`: replace hard-coded `BEST`, Shai corpus loading, and round-8 output paths with
    `--layout`, repeatable `--primary-corpus`, `--secondary-corpus`, `--out`, and `--out-json`
    options.
  - `compute.py`: replace hard-coded `CFG_E`, `DEFAULT_SEEDS`, Shai/Spanish paths, and result
    paths with corpus flags, reference/seed inputs, and output options.
  - `audit.py`: replace the hard-coded Shai probes, `REFS`, and corpus map with
    configuration/reference-driven data and generic corpus flags.
  - Corpus path arguments accept either a raw source, a corpus directory, or a parsed corpus
    directory; document the accepted forms.
  - Done when the same code paths work with two different corpus directories without editing
    source files.

- [ ] **K8 — Optimize secondary only after primary convergence**
  - Issue: TBD
  - Commit: `compute: optimize secondary only after primary convergence`
  - Remove the composite corpus from the hot loop. Main quench/anneal iterations score only the
    primary corpus/corpora.
  - After the primary runs converge, take the top primary candidates and perform constrained
    local search/ILS against the secondary corpus. Reject any candidate whose primary score
    exceeds the best primary score plus the configured sacrifice budget.
  - Report best primary, best near-tie/secondary, and the final selected layout. Composite scores,
    if still shown, are computed only for reporting, never for iteration decisions.
  - Done when a run shows one primary score per iteration and secondary improvements occur only
    in the final phase.

- [ ] **K9 — Auto-generate the seed by default**
  - Issue: TBD
  - Commit: `compute: auto-generate seed by default`
  - Make `--seed` optional. When omitted, generate a random seed, print it, and record it in the
    result JSON.
  - An explicitly supplied seed must reproduce the same run.
  - Done when two runs with the same `--seed` match and a run without `--seed` still records a
    usable seed.

- [ ] **K10 — Load reference layouts from `config/references.toml`**
  - Issue: TBD
  - Commit: `core: load reference layouts from TOML`
  - Define the reference schema as an array of tables:

    ```toml
    [[reference]]
    name = "qwerty"
    layout = "qwertyuiopasdfghjkl;zxcvbnm,./"
    ```

  - Seed the file with the layouts currently hard-coded in `compare.py`/`audit.py`.
  - Remove hard-coded `REF`/`REFS` data from active code and make `compare.py` accept `--refs`,
    defaulting to `shirolyzer/config/references.toml`.
  - Done when removing the TOML file produces a clear error and adding an entry changes the
    comparison output without code changes.

- [ ] **K11 — Append finished layouts to the reference file**
  - Issue: TBD
  - Commit: `compute: append final layout to references`
  - At the end of a successful compute run, append a new `[[reference]]` entry containing at
    minimum the generated name, layout string, primary score, secondary score, and seed.
  - Use text append or a TOML writer; never rewrite existing entries or comments. Skip duplicates
    or write a clearly unique generated name.
  - Done when the file remains valid TOML after repeated compute runs and `compare.py` sees the
    new entry.

- [ ] **K12 — Add a score-regression baseline**
  - Issue: TBD
  - Commit: `core: add score regression baseline`
  - Capture the pre-rewrite scores for the reference layouts and the current known layouts using
    the existing result JSON files where possible.
  - Add a small check script or test that recomputes those scores and fails on unintended drift.
  - Done when the baseline passes before config work begins and can be rerun after every batch.
  - Note: this is the guard for the "preserve current behaviour" requirement.

Branch exit: `core` merges to `main`; create `config` from the updated `main`.

### Branch `config`

- [ ] **G1 — Add the default YAML configuration**
  - Issue: TBD
  - Commit: `config: add default YAML configuration`
  - Create `shirolyzer/config/config.yaml` with the four required sections:
    `general`, `fingermap`, `alt_fingers`, `conditions`, plus `priority_list` and
    `finger_balance`.
  - Use this shape (comments and defaults to be filled in with the current calibrated model):

    ```yaml
    general:
      physical_layout: ansi
      primary_corpus:
        - {name: shai, weight: 100}
      secondary_corpus:
        - {name: spanish, weight: 100}
      primary_sacrifice: 0.00023   # percent of best primary score

    fingermap: |
      LR LR LM LM LM  RM RM RM RR RR
       LP LR LM LI LI  RI RI RM RR RP
        LP LR LI LI LI  RI RI RR RR RP

    alt_fingers:
      - {sequence: fw, fingers: [LM, LI]}
      - {sequence: sth, fingers: [LP, LM, LI]}

    conditions:
      - {type: scissors, distance: full, reverse: true, fingers: [LP, LR, LM, RM, RR, RP], allow: false}
      - {type: scissors, distance: half, fingers: [LP, RP], allow: false}
      - {type: scissors, distance: half, allow: true}

    priority_list:
      - sfbs-sfts
      - [scissors, redirects]
      - [rolls, alternation]
      - finger_balance

    finger_balance:
      LP: 12.5
      LR: 12.5
      LM: 12.5
      LI: 12.5
      RI: 12.5
      RM: 12.5
      RR: 12.5
      RP: 12.5
    ```

  - Port every current implicit alt-context rule and scissor/condition rule into this file so it
    is the single source of truth.
  - Done when the file contains no dead example entries and all current defaults are represented.

- [ ] **G2 — Implement the YAML loader and validator**
  - Issue: TBD
  - Commit: `config: implement YAML loader and validation`
  - Add `shirolyzer/utils/config.py` using `PyYAML`.
  - Validate: physical layout name, 30-token fingermap, corpus weight totals, known priority
    names, non-negative percentages, valid condition fields, and alt sequences using valid QWERTY
    position labels.
  - Support a scalar or list for corpora; normalise both.
  - Precedence: command-line flags override YAML values, which override built-in defaults.
  - Done when invalid configs produce actionable errors and a valid config loads identically from
    the default path and `--config PATH`.

- [ ] **G3 — Drive default fingering from the configured fingermap**
  - Issue: TBD
  - Commit: `fingers: derive defaults from configured fingermap`
  - Remove the hard-coded per-key default fingers from `fingers.py`; read them from the loaded
    config.
  - Keep geometry, travel, strength, and other physical constants in code.
  - Ensure computed fingerings never violate the fingermap unless allowed by an alt-finger
    sequence.
  - Done when changing one fingermap entry changes the default fingering and audit output.

- [ ] **G4 — Support configurable alt-finger sequences**
  - Issue: TBD
  - Commit: `scorer: support configurable alt-finger sequences`
  - Port the current hard-coded `_ALT_CONTEXTS`, `_ALT_BLOCKS`, per-key alt premiums, and related
    gates into the default `alt_fingers` list.
  - Support sequences of arbitrary length (bigram, trigram, quatrigram, and longer) using QWERTY
    physical positions.
  - Apply declared alts during scoring/search and make them override the fingermap only where
    declared.
  - Done when adding a new alt sequence changes analysis and generation without source edits.

- [ ] **G5 — Implement the conditions engine and override order**
  - Issue: TBD
  - Commit: `objective: enforce configured conditions`
  - Implement structured conditions for at least the current scissor classes: full/half,
    regular/reverse, optional finger lists, and allow/deny.
  - Preserve the spec's override order: entries higher in the list take precedence over lower
    entries. Add a regression test using the spec's half-scissor example in both orders.
  - Route existing impossibility rules through this engine instead of ad-hoc checks.
  - Done when a condition change alters the allowed fingering set and audit catches violations.

- [ ] **G6 — Implement tiered priority weights and alternation**
  - Issue: TBD
  - Commit: `objective: apply tiered priority weights`
  - Rename the priority field `alts` to `alternation` and add a hand-alternation statistic to the
    scorer/analyzer.
  - Convert `priority_list` tiers into category multipliers. Items in the same tier share equal
    weight; higher tiers receive larger multipliers.
  - Default multipliers must keep the current calibrated scores reproducible (the regression
    baseline from K12 is the acceptance test).
  - Done when reordering the priority list demonstrably changes the search trade-offs while the
    default config still passes the regression check.

- [ ] **G7 — Load the shared YAML config in all entry points**
  - Issue: TBD
  - Commit: `core: load shared YAML config in entry points`
  - Wire `analyze.py`, `compute.py`, and `audit.py` to load the config and use it for corpora,
    physical layout, fingermap, alt fingers, conditions, priority tiers, and finger balance.
  - Keep CLI overrides functional.
  - Done when one config edit affects analysis, generation, and audit consistently.

- [ ] **G8 — Verify the default config preserves behaviour**
  - Issue: TBD
  - Commit: `config: verify default configuration preserves scores`
  - Run the K12 regression check against the ported default config.
  - Investigate and document any unavoidable deviation; do not silently accept score drift.
  - Done when reference rankings are unchanged and numeric scores match within the agreed
    tolerance, or a documented follow-up question exists.

Branch exit: `config` merges to `main`; create `xkb` from the updated `main`.

### Branch `xkb`

- [ ] **X1 — Add the layout-to-XKB symbol generator**
  - Issue: TBD
  - Commit: `xkb: add layout-to-symbols generator`
  - Add `shirolyzer/utils/xkb.py` that converts a 30-character layout string plus a layout name
    into the XKB symbols format used by `xkb/mine`.
  - Preserve the current punctuation mapping and Option/ISO-key handling.
  - Include generated comments with the layout name and grid.
  - Done when generating from the existing cfgE layout produces a file equivalent to
    `xkb/mine` apart from comments/metadata.

- [ ] **X2 — Add the `--xkb` flag to compute**
  - Issue: TBD
  - Commit: `compute: add xkb output flag`
  - Add `--xkb` (default false) and `--xkb-name` (default a safe generated name) to
    `compute.py`.
  - When set, write the final selected layout to `xkb/<name>` after a successful run.
  - Do not overwrite `xkb/mine` unless `--xkb-name mine` is passed explicitly.
  - Done when a compute run with `--xkb` creates the file and a run without the flag creates
    nothing.

- [ ] **X3 — Add a standalone generator/installer for existing layouts**
  - Issue: TBD
  - Commit: `xkb: add standalone layout generator and installer`
  - Add `shirolyzer/xkb.py` with `--layout`, `--name`, `--out`, and optional `--install`.
  - Update `xkb/install.sh` to accept an optional symbols/name argument while keeping the current
    default behaviour.
  - Done when an already-generated layout can be converted and installed without running the
    annealer.

- [ ] **X4 — Validate and document the XKB workflow**
  - Issue: TBD
  - Commit: `xkb: validate generated symbols and document usage`
  - Run `bash -n xkb/install.sh`, generate a known layout, and compare against `xkb/mine`.
  - If an X display is available, compile with `xkbcomp` and confirm the keymap loads; otherwise
    record the dry-run validation.
  - Update `xkb/TODO.txt` and the relevant README sections.
  - Done when the TODO is resolved and the documented commands work as written.

Branch exit: `xkb` merges to `main`; create `calibration` from the updated `main`.

### Branch `calibration`

- [ ] **T1 — Add the compute time-budget flag**
  - Issue: TBD
  - Commit: `compute: add time budget flag`
  - Add `--time-budget` with a documented unit (seconds or `30m`-style strings) and graceful
    stopping between runs/ILS rounds. Always save the best result found so far.
  - Record completed runs, iterations, ILS rounds, elapsed time, and the active seed in the output
    JSON.
  - Done when a very short budget still produces a valid result and a longer budget performs more
    work without exceeding the cap by more than one in-flight operation.

- [ ] **T2 — Run and record the tuning experiments**
  - Issue: TBD
  - Commit: `results: add compute runtime calibration`
  - Compare multiple time budgets (for example 2, 5, 10, 20, 30, and 60 minutes) and independent
    sweeps of iterations, runs, and ILS rounds.
  - Use fixed seeds for comparisons and record primary score, secondary score, final layout, and
    wall-clock time under `shirolyzer/results/calibration/`.
  - Identify the point of diminishing returns for each parameter.
  - Done when the calibration data is committed and summarised in a short report.

- [ ] **T3 — Choose and apply the new compute defaults**
  - Issue: TBD
  - Commit: `compute: update search defaults from calibration`
  - Increase iterations well above 60k, runs above 24, and ILS above 6 according to T2.
  - Set the default time budget if the data supports it; otherwise keep the flag optional.
  - Update config comments, help text, and docs with the chosen values and rationale.
  - Done when a default run uses the new values and the reason for each value is recorded.

- [ ] **T4 — Run the post-rewrite validation and refresh references**
  - Issue: TBD
  - Commit: `results: record post-rewrite validation run`
  - Run the full compute pipeline with the new defaults, audit the winner, and append the final
    layout(s) to `config/references.toml`.
  - Compare against pre-rewrite reference results and record any improvements or regressions.
  - Done when the validation artifacts are saved and references include the new layout.

Branch exit: `calibration` merges to `main`; create `docs` from the updated `main`.

### Branch `docs`

- [ ] **D1 — Update analyzer documentation and usage examples**
  - Issue: TBD
  - Commit: `docs: update analyzer documentation`
  - Update `shirolyzer/README.md` and the root `README.md` for the new structure, YAML config,
    corpus commands, two-phase search, references TOML, and XKB workflow.
  - Remove references to `core/`, `analysis/`, `corpus/`, `scorer2`, `search2`, and old script
    names.
  - Done when every documented command matches the implemented CLI.

- [ ] **D2 — Final stale-reference and import cleanup**
  - Issue: TBD
  - Commit: `docs: finalize rewrite cleanup`
  - Run the section 4 quality gates, fix any remaining active stale references, and confirm
    `legacy/` is not imported.
  - Update this tasklist to checked state and record the final branch/issue status.
  - Done when all gates pass and the tasklist reflects the completed rewrite.

Branch exit: `docs` merges to `main`; the rewrite is complete.

---

## 6. Spec coverage map

| Spec item | Task(s) |
|---|---|
| Rename `analyze_shai.py` → `analyze.py` | K2 |
| Rename `anneal_shai.py` → `compute.py` | K2 |
| Rename `audit_shai.py` → `audit.py` | K2 |
| Move/rename corpus build scripts | C1 |
| `parsed/` sub-directory for both corpora | C2 |
| Corpus scripts download, parse, and delete source | C3, C4 |
| Compare references come from a file | K10 |
| Compute appends layout to references | K11 |
| `--xkb` flag, default false | X2 |
| Standalone XKB installer/generator from existing layout | X3 |
| No `core/` directory; analyzer files directly under `shirolyzer/` | K1, K2 |
| Corpus-independent analyzer/compute/audit | K7 |
| Secondary only after primary convergence | K8 |
| Higher iteration default | T3 |
| Higher run default, determined experimentally | T2, T3 |
| Higher ILS default | T2, T3 |
| Auto-generated seed, CLI override | K9 |
| `utils/` contains ansi, comfort, fingers, search, scorer | K3, K4, K5 |
| ANSI geometry in its own one-word physical-layout package | K5 |
| `config/` with general config, fingermap, alt list, conditions | G1, G2 |
| Config used by both analyzer and generator | G7 |
| Conditions respect override order | G5 |
| Priority list tiers/blending | G6 |
| `alts` priority means hand alternation, renamed `alternation` | G6 |
| Preserve current calibrated behaviour by default | K12, G8 |
| Runtime time-budget parameter | T1, T2 |
| YAML config format | G1, G2 |

---

## 7. Questions and deferred decisions

The major planning questions were resolved in section 1. The following are minor and do not block
the implementation tasks above. Current defaults are listed so work can continue.

1. **`README.md.bak`:** keep it as a historical backup, archive it, or delete it?
   Current default: leave it untouched; the rewrite only updates `README.md`.
2. **`shirolyzer/legacy/`:** keep it frozen, migrate it to the new package layout, or archive it
   outside the active tree?
   Current default: keep it frozen and ensure active code never imports it. Legacy imports may
   remain stale by design.
3. **Default `--xkb-name`:** use a generated name based on the run/seed, or a fixed name such as
   `shiro`?
   Current default: generated name, so existing `xkb/mine` is never overwritten accidentally.
4. **Exact score-equivalence tolerance:** the regression check needs a numeric tolerance for
   floating-point and refactor drift. Current default: exact match where feasible, otherwise a
   documented tolerance agreed before G8 is checked off.

If any answer changes an assumption used by a task, update this file and the corresponding GitHub
issue before starting that task.
