# Shirolyzer Rust Rewrite — Tasklist and Implementation Plan

Source specification: `RUST_REWRITE_SPEC_LIST.txt` (must remain unchanged).
Status: complete and published. Rust is the primary implementation; the shipped
backend is CPU-only and the defaults are tuned. The user authorized publication,
so the branches, per-task commits, and GitHub issues are live (section 3).
Last updated: 2026-10-30.

Post-rewrite note: the follow-up takeover in `RUST_TAKEOVER_TASKLIST.md` moved
the crate from `rust/` to `shirolyzer/` and removed the Python runtime from the
shipped tree.  Paths written as `rust/...` below are historical; the current
runtime layout and commands are documented in `README.md` and
`shirolyzer/README.md`.  The Rust implementation remains the primary one.

This document is the implementation plan and progress tracker for the Rust rewrite
of `shirolyzer`. It records the decisions, the git roadmap, the quality gates, and
the tasks for each phase of the specification: simple port, CPU optimization,
optional GPU acceleration, and generation-parameter calibration.

The Python implementation is the behaviour reference. It is not deleted by this
rewrite; it stays in place, frozen, until the Rust implementation passes parity.

---

## 1. Locked decisions

1. **Location and name:** a single Rust binary crate lives in `rust/` and builds
   the `shirolyzer` binary. The crate is not a workspace with several crates; one
   small binary keeps the port short and readable. Shared runtime data stays in
   `shirolyzer/config/` (YAML, parameters, references, baseline), and parsed
   corpora stay in `corpora/*/parsed/`.
2. **Python is the parity oracle, not a runtime dependency.** During the rewrite
   both implementations are runnable. The final documentation declares the Rust
   binary the primary implementation. Whether to delete the Python tree is a
   deferred user decision; it does not block any phase.
3. **CLI parity.** Port every runtime command: `compute`, `analyze`, `render`,
   `xkb`, `compare`, `audit`, and `check-baseline`. Keep the existing flag names
   and defaults where they are meaningful. `--device cpu|gpu` is added for phase 3.
   Development-only tools (`shirolyzer/fitting/`) are not ported.
4. **Numerical model:** use `f64` throughout, matching NumPy's `float64`. Tie
   breaking in table construction follows the Python loop order exactly (finger
   loops in `P, R, M, I` order).
5. **Parity definition:**
   - Reference bigram/trigram costs match `score_baseline.json` within `1e-9`.
   - Audit result is exactly 166/166 enforced rulings.
   - `analyze` scores and category/trigram/usage tables match Python within `1e-9`.
   - `compute` is compared on identical layouts and identical seeds; the search
     algorithm and RNG are shared between CPU and GPU modes so that a seed gives
     the same result on both devices.
   - Python and Rust search runs are *quality-comparable*, not seed-identical:
     the Python `random.Random` and the Rust RNG are different generators. The
     tasklist explicitly records this.
6. **GPU backend:** `wgpu` (Vulkan on this machine's AMD Radeon RX 6800M) is the
   first choice because it is pure Rust, cross-platform, and available. The GPU
   path is optional at runtime and must fall back to CPU with a clear message when
   no adapter is available. If benchmarked GPU throughput cannot beat the optimized
   CPU, the GPU mode is removed or left disabled and the result is documented, as
   the specification allows.
7. **RNG:** use `rand_chacha::ChaCha8Rng` (or `rand::rngs::StdRng` if the
   dependency footprint matters) with an explicit seed. All stochastic decisions
   are made in one shared search module; the GPU evaluator only replaces score
   computation, never the sequence of decisions.
8. **Objective:** port the existing pattern model unchanged. Optimization must not
   change scores. New search heuristics are only allowed in phase 2 after a parity
   test proves they produce the same objective value for the same layout.
9. **Git workflow:** branches are created from `main` in the order below. No branch
   is merged until its exit condition passes. Publishing branches and GitHub issues
   was deferred until the user explicitly authorized full publication on
   2026-10-30; the branches, per-task commits, and issues are now published.
10. **Data formats:** keep YAML for user config, TOML for references, and JSON for
    corpora/baseline/audit output. Rulings data is exported once from Python into
    `shirolyzer/config/rulings.json` so the Rust audit evaluates the same data
    without re-implementing a Python data module.

## 2. Assumptions and deferred questions

- The Rust crate may use `clap`, `serde`, `serde_json`, `serde_yaml`, `toml`,
  `rand`, `rayon`, and `wgpu`. All are widely available; the environment already
  has the crates.io index cached and network access.
- The GPU target is the Vulkan device present on this machine. CUDA-only paths
  are not considered.
- Raw corpus parsing (`--source`, `--url`, `.xz`, `.zip`) is part of `Corpus::load`
  and is ported, but the Python corpus *parser scripts* stay Python-only; the
  rewrite consumes their `parsed/*.json` output. This keeps the Rust port focused
  on the analyzer/generator. If full corpus-parser parity is wanted later, it is a
  separate, non-blocking task.
- Deleting the Python implementation after Rust parity is a deferred question. It
  does not block phases 1-4 because both trees can coexist.
- The GitHub issues and branches were authorized and created on 2026-10-30. The
  branch names below are the published branch names.

## 3. Git roadmap

Branches, in order, from an up-to-date `main`:

| Order | Branch | Scope | Exit condition |
|---|---|---|---|
| 0 | `rust-plan` | This tasklist, baseline fixtures, crate skeleton | Plan committed; `cargo check` passes on an empty binary |
| 1 | `rust-base` | Simple Python-to-Rust port of all runtime commands | `cargo test`, parity fixtures, audit 166/166, baseline match |
| 2 | `rust-cpu` | Delta scoring, faster quench/ILS, parallelism, profiling | Benchmarks recorded; no score changes; parity tests still pass |
| 3 | `rust-gpu` | `wgpu` evaluator, device flag, CPU/GPU parity harness | `--device gpu` matches CPU; speedup measured; fallback documented |
| 4 | `rust-tuning` | Parameter sweeps, new defaults, diversity report | Plateau reached; report written; defaults updated |
| 5 | `rust-docs` | READMEs, usage, removal/stale-reference cleanup | Docs match the binary; all gates pass |

Planned issues (labels: `rewrite`, `rust`, plus the branch name; milestone = phase):

| Issue | Branch | Title |
|---|---|---|
| [#40](https://github.com/akai-hana/shiro-layout/issues/40) | `rust-plan` | R0.1 — Write the plan and freeze the baseline |
| [#41](https://github.com/akai-hana/shiro-layout/issues/41) | `rust-plan` | R0.2 — Scaffold the crate |
| [#42](https://github.com/akai-hana/shiro-layout/issues/42) | `rust-base` | R1.1 — Paths, YAML config, and validation |
| [#43](https://github.com/akai-hana/shiro-layout/issues/43) | `rust-base` | R1.2 — Geometry, model, gates, and conditions |
| [#44](https://github.com/akai-hana/shiro-layout/issues/44) | `rust-base` | R1.3 — Corpus and scorer tables |
| [#45](https://github.com/akai-hana/shiro-layout/issues/45) | `rust-base` | R1.4 — Search primitives and small commands |
| [#46](https://github.com/akai-hana/shiro-layout/issues/46) | `rust-base` | R1.5 — Compute and diversity |
| [#47](https://github.com/akai-hana/shiro-layout/issues/47) | `rust-base` | R1.6 — Audit and regression baseline |
| [#48](https://github.com/akai-hana/shiro-layout/issues/48) | `rust-base` | R1.7 — Simple-port parity report |
| [#49](https://github.com/akai-hana/shiro-layout/issues/49) | `rust-cpu` | R2.1 — Benchmark harness |
| [#50](https://github.com/akai-hana/shiro-layout/issues/50) | `rust-cpu` | R2.2 — Incremental delta scoring |
| [#51](https://github.com/akai-hana/shiro-layout/issues/51) | `rust-cpu` | R2.3 — Faster quench and ILS |
| [#52](https://github.com/akai-hana/shiro-layout/issues/52) | `rust-cpu` | R2.4 — Parallel seeds |
| [#53](https://github.com/akai-hana/shiro-layout/issues/53) | `rust-cpu` | R2.5 — Profile and record |
| [#54](https://github.com/akai-hana/shiro-layout/issues/54) | `rust-gpu` | R3.1 — Feasibility probe |
| [#55](https://github.com/akai-hana/shiro-layout/issues/55) | `rust-gpu` | R3.2 — Batched evaluator and device selection |
| [#56](https://github.com/akai-hana/shiro-layout/issues/56) | `rust-gpu` | R3.3 — Decision parity |
| [#57](https://github.com/akai-hana/shiro-layout/issues/57) | `rust-gpu` | R3.4 — Benchmark and decision |
| [#58](https://github.com/akai-hana/shiro-layout/issues/58) | `rust-tuning` | R4.1 — Iteration sweep |
| [#59](https://github.com/akai-hana/shiro-layout/issues/59) | `rust-tuning` | R4.2 — Run/seed sweep |
| [#60](https://github.com/akai-hana/shiro-layout/issues/60) | `rust-tuning` | R4.3 — ILS/polish sweep |
| [#61](https://github.com/akai-hana/shiro-layout/issues/61) | `rust-tuning` | R4.4 — Combined validation |
| [#62](https://github.com/akai-hana/shiro-layout/issues/62) | `rust-tuning` | R4.5 — Diversity/near-equal report |
| [#63](https://github.com/akai-hana/shiro-layout/issues/63) | `rust-tuning` | R4.6 — Apply defaults |
| [#64](https://github.com/akai-hana/shiro-layout/issues/64) | `rust-docs` | R5.1 — Documentation |
| [#65](https://github.com/akai-hana/shiro-layout/issues/65) | `rust-docs` | R5.2 — Final cleanup and completion record |

Branch rule: one task equals one commit; commit messages use the task title after
the branch prefix (`rust-base: ...`). Do not modify `RUST_REWRITE_SPEC_LIST.txt`.

## 4. Quality gates

Run at the end of every branch:

```bash
cd rust
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --release -- check-baseline
cargo run --release -- analyze ";ntgfjoehkrpmscuaydilbqvwxz,./" --out /tmp/rs-analysis.txt --out-json /tmp/rs-analysis.json
cargo run --release -- compare
cargo run --release -- audit --out /tmp/rs-audit.txt --out-json /tmp/rs-audit.json
```

Parity gate (development only, using the Python oracle):

```bash
.venv/bin/python -m shirolyzer check-baseline
.venv/bin/python -m shirolyzer analyze LAYOUT --out-json /tmp/py.json
cargo run --release -- analyze LAYOUT --out-json /tmp/rs.json
# compare scores/categories/usage within 1e-9
```

Performance gate:

```bash
cd rust && cargo bench
```

Record wall-clock, iterations/second, and score equivalence in
`shirolyzer/results/rust/` (local) and commit only the summary report.

## 5. Tasks

### Phase 0 — Planning and baseline

- [x] **R0.1 — Write the plan and freeze the baseline**
  - Files: `RUST_REWRITE_TASKLIST.md`, `shirolyzer/config/rulings.json`.
  - Capture the Python baseline before touching Rust: audit 166/166, reference
    bigram/trigram costs, analyze categories, and the current compute defaults.
  - Done when this document exists and the baseline commands are recorded.

- [x] **R0.2 — Scaffold the crate**
  - Files: `rust/Cargo.toml`, `rust/src/main.rs`, `rust/.gitignore`.
  - Add dependencies and a clap skeleton with all seven subcommands.
  - Done when `cargo check` passes and every subcommand responds to `--help`.

### Branch `rust-base` — simple, readable port

- [x] **R1.1 — Paths, YAML config, and validation**
  - Port `paths.py` and `config.py`; accept the same sections, corpus forms,
    conditions, priorities, alt-finger gates, and parameter-file lookup.
  - Done when the shipped config loads and invalid configs produce actionable
    errors.

- [x] **R1.2 — Geometry, model, gates, and conditions**
  - Port `ansi.py`, `fingers.py`, `comfort.py`, `gates.py`, and `conditions.py`.
  - Implement the approved-point convex hull for `REACH` (equivalent to
    `Delaunay.find_simplex >= 0`), `UNLISTED`, pair features, key costs, pair
    parts, and pair tables.
  - Done when `E`, `PT`, `T2`, `strain`, `REACH`, and the explicit gate masks match
    the Python values for the shipped config.

- [x] **R1.3 — Corpus and scorer tables**
  - Port `corpus.py` (parsed JSON plus raw `.txt`/`.xz`/`.zip`) and
    `scorer.build_tables` for bigram/trigram costs and chosen fingerings.
  - Done when reference scores match `score_baseline.json` within `1e-9`.

- [x] **R1.4 — Search primitives and small commands**
  - Port `search.py` (dead keys, slots, evaluator, quench, anneal, perturb) plus
    `analyze`, `render`, `compare`, and `xkb`.
  - Done when analyze output scores/categories/usage match Python and the
    generated XKB body matches `xkb/mine`.

- [x] **R1.5 — Compute and diversity**
  - Port `compute.py` and `diversity.py`: reference/random seeds, quench, anneal,
    ILS, optional polish, secondary phase under the sacrifice budget, diversity
    selection, reference appending, XKB writing, and text/JSON output.
  - Done when a short run produces the same phases and valid-layout structure as
    Python and appends valid TOML.

- [x] **R1.6 — Audit and regression baseline**
  - Export the rulings once (R0.1), port `fitlib.eval_ruling` and the audit probe
    checks, and implement `check-baseline`.
  - Done when `audit` reports 166/166 and `check-baseline` passes with all
    references.

- [x] **R1.7 — Simple-port parity report**
  - Add fixtures/tests for scores, table shapes, gate masks, diversity, and XKB.
  - Record any unavoidable difference (RNG only) in `shirolyzer/results/rust/`.
  - Done when all gates pass and the port is declared stable.

### Branch `rust-cpu` — CPU optimization

- [x] **R2.1 — Benchmark harness**
  - Add repeatable micro-benchmarks for table build, score, quench, anneal, and a
    short compute run.
  - Done when before/after numbers are recorded in the report.

- [x] **R2.2 — Incremental delta scoring**
  - Precompute per-letter trigram incidence; score a swap/cycle by updating only
    affected trigram entries. Keep a full-score fallback for verification.
  - Done when `delta_score == full_score` for randomized layouts within `1e-12`
    and analyze/KPI scores are unchanged.

- [x] **R2.3 — Faster quench and ILS**
  - Use best-improvement/delta move evaluation, avoid rebuilding move lists, and
    cache the current score. Preserve objective values and valid-layout quality.
  - Done when short-run wall clock is materially lower with no score regression.

- [x] **R2.4 — Parallel seeds**
  - Run independent seeds and ILS candidates with rayon while keeping each seed's
    RNG and result deterministic.
  - Done when output is independent of thread count for a fixed seed set.

- [x] **R2.5 — Profile and record**
  - Remove per-score allocations, flatten hot lookups, and record sampling profile
    plus before/after throughput.
  - Done when no obvious hot spot remains and the CPU mode is the baseline for
    phase 3.

### Branch `rust-gpu` — optional GPU acceleration

- [x] **R3.1 — Feasibility probe**
  - Create a `wgpu` adapter and prototype a kernel that scores a batch of
    candidate permutations exactly (f64 emulation or documented precision).
  - Done when the kernel returns values equal to CPU within `1e-9`.

- [x] **R3.2 — Batched evaluator and device selection**
  - Introduce `--device cpu|gpu` (default `gpu` when an adapter is available) and
    route batch candidate evaluation through the selected backend.
  - Done when missing GPU/adapter falls back to CPU with a warning.

- [x] **R3.3 — Decision parity**
  - Ensure the GPU only replaces score evaluation, never RNG or move order. Run
    identical seeds and assert identical results/decisions.
  - Done when CPU and GPU outputs match for all test seeds and layouts.

- [x] **R3.4 — Benchmark and decision**
  - Compare wall-clock on equal work. If GPU is not several times faster, leave
    the default on CPU (or remove GPU) and record why in the report.
  - Done when the report states the measured speedup and the shipped default.
  - Decision: CPU only. `--device gpu` falls back with a warning. Reason: WGSL
    has no `f64`, the search is sequential, and the CPU already reaches the
    tuning plateau in about a minute. See
    `shirolyzer/results/rust/GPU_FEASIBILITY.md`.

### Branch `rust-tuning` — generation-parameter calibration

- [x] **R4.1 — Iteration sweep**
  - Sweep anneal iterations (e.g. 100k, 250k, 500k, 1M, 2M) at fixed seeds and a
    time budget; record primary/secondary scores and wall time.
  - Done when further increases change the best score by less than the validity
    tolerance.

- [x] **R4.2 — Run/seed sweep**
  - Sweep random restarts and total seeds until the best score stops improving.
  - Done when the plateau and its sample variability are recorded.

- [x] **R4.3 — ILS/polish sweep**
  - Sweep ILS rounds and polish rounds; track primary, secondary, and diversity.
  - Done when additional rounds no longer produce meaningful improvement.

- [x] **R4.4 — Combined validation**
  - Run the chosen values, audit the winners, and compare to the Python
    calibration report (`shirolyzer/results/calibration/REPORT.md`).
  - Done when the final artifacts and score summary are saved.

- [x] **R4.5 — Diversity/near-equal report**
  - If materially different layouts remain within tolerance, list them with score
    deltas and key-position distances, and describe which criteria are missing or
    ambiguous. This is the report requested by the specification note.
  - Done when `shirolyzer/results/rust/DIVERSITY_REPORT.md` is written.

- [x] **R4.6 — Apply defaults**
  - Update Rust defaults, config comments, and docs; explain each chosen value.
  - Done when a default run uses the new values and the rationale is recorded.

### Branch `rust-docs`

- [x] **R5.1 — Documentation**
  - Update root `README.md`, `shirolyzer/README.md`, and `justfile` for the Rust
    binary, device flag, parity checks, and new defaults.
  - Done when every documented command runs as written.

- [x] **R5.2 — Final cleanup and completion record**
  - Remove stale references, mark tasks complete, and record branch/issue status,
    benchmarks, parity results, and the Python-removal decision.
  - Done when all quality gates pass and this tasklist reflects the final state.

## 6. Spec coverage map

| Spec item | Task(s) |
|---|---|
| Plan the rewrite and git roadmap | R0.1, section 3 |
| Simple, readable Python-to-Rust port that works | R0.2, R1.1-R1.7 |
| Preserve calibrated behaviour | R1.2, R1.3, R1.6, R1.7 |
| CPU-only optimization after the simple base | R2.1-R2.5 |
| Introduce GPU alongside CPU, default GPU if faster | R3.1-R3.4 |
| 100% CPU/GPU parity | R1.7, R3.3 |
| Switch devices with a flag | R3.2 |
| Leave pure CPU if GPU is not viable | R3.4 |
| Tune iterations, runs, ILS, and polish after speedups | R4.1-R4.4 |
| Find the point where more work stops helping | R4.1-R4.4 |
| Report uneven near-equal distributions | R4.5 |
| Document final results and defaults | R4.6, R5.1, R5.2 |

## 7. Parity and performance acceptance criteria

1. **Numeric parity:** reference bigram/trigram costs and analyze scores within
   `1e-9`; audit 166/166; XKB body identical.
2. **Objective parity:** for every tested layout and parameter file, the Rust
   objective equals Python within `1e-12` (table construction uses the same
   formulas and loop order).
3. **Search quality:** with the same iteration/restart/ILS budget, the Rust search
   reaches at least the Python best primary score on the calibration seed set.
4. **CPU speed:** phase 2 must improve score evaluations/second by a material
   factor (target >= 10x over the simple port; actual number recorded).
5. **GPU speed:** phase 3 must be measurably faster than the optimized CPU on equal
   work (target >= 3x). If it is not, CPU remains the default and a report explains
   why.
6. **Tuning plateau:** the final defaults must be at a point where doubling the
   parameter changes the best primary score by less than `0.002` and does not
   produce a materially better layout.

## 8. Progress log

- 2026-10-29: Read `RUST_REWRITE_SPEC_LIST.txt`; inspected the Python reference,
  config, corpora, calibration report, tests, and the machine. Confirmed Rust
  1.99 and Vulkan (AMD RX 6800M) are available. Wrote this plan and exported the
  rulings data. Next: crate skeleton and the simple port.

- 2026-10-30: Completed the simple port and its parity gates. `analyze` JSON,
  render, XKB, compare, audit (166/166), and `check-baseline` match Python;
  only the exploratory RNG family differs. Added randomized delta-vs-full
  scoring tests, command tests, and reference assertions.
- 2026-10-30: CPU phase validated. Delta-scored quench is 4.6x faster than a
  full-score quench, 16-thread seed evaluation is 2.0x faster than one thread
  with byte-identical results, and end-to-end Rust is about 39x faster than the
  Python oracle on a small run. See `shirolyzer/results/rust/BENCHMARKS.md`.
- 2026-10-30: GPU phase closed as not viable for this rewrite: Vulkan and f64
  hardware exist, but WGSL/WGPU has no f64 and the sequential delta search does
  not map to independent batches without breaking decision parity. `--device
  gpu` ships as an explicit CPU fallback.
- 2026-10-30: Tuned with qwerty-only references so the search had to rediscover
  shiro. Every iteration/restart/ILS/polish configuration converged to the same
  layout; the knee is at or below 25k iterations, 4 restarts, and 2 ILS rounds.
  Shipped defaults are `iters=200000`, `runs=28`, `ils=8`, `polish=0`. See
  `shirolyzer/results/rust/TUNING_REPORT.md`.
- 2026-10-30: Wrote the near-equal distribution report. The closest different
  layout is 0.00158 worse with ten key changes; it was not selected because it
  had no constrained-secondary score. See
  `shirolyzer/results/rust/DIVERSITY_REPORT.md`.
- 2026-10-30: Updated the root README, analyzer README, and justfile for the
  Rust binary, `--device`, parity recipes, and new defaults. `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test --release`, and
  `check-baseline` all pass.
- 2026-10-30: Finalized the completion record in
  `shirolyzer/results/rust/SUMMARY.md` and linked it from the root README.
  Tightened CLI/JSON parity: default `analyze`/`audit`/`compute` text outputs use
  `.txt` like Python, trigram-guide counts and audit corpus totals serialize as
  integers, and a default `audit` records `null` for `config`. Re-ran fmt, clippy,
  release tests, check-baseline, analyze, compare, and audit; all gates pass.
- 2026-10-30: The user authorized full publication. Reconstructed the per-task
  commit history on the six documented branches from the session snapshot
  trees, pushed every branch, fast-forwarded `main`, and opened issues
  #40-#65 (labels `rewrite`, `rust`, and the phase; one milestone per branch).
- 2026-10-30: Started and completed the follow-up takeover epic
  (`RUST_TAKEOVER_TASKLIST.md`): benchmarked the Python oracle, moved the crate
  into `shirolyzer/`, removed the Python runtime, retained the corpus parsers
  and refitting tools, and updated the docs and `justfile` for the new layout.
