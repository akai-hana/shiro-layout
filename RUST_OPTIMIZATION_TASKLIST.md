# Shirolyzer Rust Optimization — Tasklist and Implementation Plan

Status: in progress. This is the follow-up optimization epic to
`RUST_TAKEOVER_TASKLIST.md`. The Rust binary is already the only runtime; this
plan makes the generator and analyzer faster without changing the objective,
the calibrated defaults, the corpora, or the audit rulings.

Source history: `RUST_REWRITE_SPEC_LIST.txt` remains unchanged. The
pre-optimization baseline is
[`shirolyzer/results/rust/OPT_BASELINE.md`](shirolyzer/results/rust/OPT_BASELINE.md).
Last updated: 2026-10-30.

---

## 1. Locked decisions

1. **No objective changes.** Scores, table values, defaults, config/data
   paths, and audit rulings stay fixed. Optimization is an implementation
   change, not a model change.
2. **Generator first, analyzer second, pruning last.** The generator is the
   long-running path and has the clearest headroom; the analyzer is short but
   repeated often; pruning is only attempted after the safe wins are measured.
3. **Dual-delta secondary phase.** The constrained secondary search must use
   incremental primary and secondary scores instead of full rescoring. It must
   reproduce the full-score objective within `1e-12` and keep the same
   constrained optimum on the benchmark seeds.
4. **Deterministic parallel search.** Parallel ILS/polish candidates use
   per-candidate RNG streams derived from the master seed and candidate index.
   Output must be independent of thread count. Serial and parallel runs are
   quality-comparable, not necessarily bit-identical, because the candidate
   RNG streams are deliberately independent.
5. **Internal representations are free to change.** Key ids may become `u8`,
   the delta cross table may become dense, and temporary arrays may be reused,
   provided randomized delta-vs-full tests still pass within `1e-12`.
6. **Analyzer parallelism is deterministic.** Independent table rows and report
   passes may run in parallel. Text output order stays fixed; JSON values and
   counts stay byte-compatible after serialization.
7. **Pruning is opt-in until proven.** Any pruning that can change the search
   path is prototyped behind a flag or a separate function, measured against
   the benchmark seed set, and shipped only if the selected layout and scores
   are unchanged. Otherwise it is documented and reverted or left disabled.
8. **Git workflow.** One task equals one commit; commit messages use the branch
   prefix (`opt-generator: OPT.T1.1 — ...`). Branches are stacked from `main`
   in the order below and fast-forwarded only after their exit conditions pass.
   Every branch, issue, and label is published as part of the epic.
9. **No unrelated edits.** The user's pending corpus README changes and
   `corpora/spanish/TODO.txt` are not part of this epic and must not be
   committed here.
10. **Rollback.** Before the final tag, any branch can be abandoned; `main`
    still points at `9e6e1d7`. After the tag, `git checkout rust-optimization^`
    recovers the pre-optimization binary. No data or history is deleted.

## 2. Baseline evidence

Full details are in [`OPT_BASELINE.md`](shirolyzer/results/rust/OPT_BASELINE.md).
Summary at `9e6e1d7`, AMD Ryzen 9 6900HX, Rust 1.99.0:

| Measurement | Baseline |
|---|---:|
| `analyze` median | 0.045 s |
| Small `compute` wall clock | 3.92 s |
| Delta-table build | 41.5 ms |
| Full score | 21.1 us/op |
| Single-swap delta | 15.8 us/op |
| Delta quench | 221.3 ms |
| Full-score quench | 1,006.8 ms |
| Anneal | 61,344 iters/s |

Profile hotspots:

- Generator: `Local::delta_cross` 61.7%, `Local::delta` 28.9%,
  `DeltaTables::build` 3.8%, `memcpy` 2.5%.
- Analyzer: `Model::tri_many` 21.8%, `fingering_guide` 15.0%, `memset` 6.1%,
  allocation/page-fault path 4.5%.

Targets (not gates that may force risky changes):

| Metric | Target |
|---|---:|
| Small `compute` | 1.5-2.5x faster |
| Default `compute` | material reduction in wall clock |
| `analyze` | 1.5-2x faster |
| Full-score quench replacement | >= 3x faster than `quench_generic` |
| Score/selection parity | exact within `1e-12`; same selected layout |
| Thread-count determinism | byte-identical JSON for fixed seeds |

## 3. Git roadmap

Branches are stacked from `main` in this order. Each row's base is the previous
row's completed branch.

| Order | Branch | Scope | Exit condition |
|---|---|---|---|
| 0 | `opt-bench` | Baseline report, profiling, harness check | Baseline committed; numbers reproducible |
| 1 | `opt-plan` | This tasklist, milestone, labels, issues | Plan committed; issues #76-#89 published |
| 2 | `opt-generator` | Dual-delta secondary, parallel ILS/polish, delta micro-optimizations | Generator benchmarks improve; parity and determinism pass |
| 3 | `opt-analyzer` | Parallel table build, concurrent report passes, allocation trimming | Analyze benchmark improves; outputs match |
| 4 | `opt-prune` | Aggressive pruning audit, prototype, decision | Pruning is either safe and shipped or documented and disabled |
| 5 | `opt-verify` | Full gates, final benchmarks, completion, tag | All gates pass; `main` fast-forwards; tag published |

Published issues (milestone `Rust optimization`; labels `rewrite`, `rust`,
`optimization`, and the branch label):

| Issue | Branch | Task |
|---|---|---|
| [#76](https://github.com/akai-hana/shiro-layout/issues/76) | `opt-bench` | OPT.T0.1 — Freeze optimization baseline and profiling evidence |
| [#77](https://github.com/akai-hana/shiro-layout/issues/77) | `opt-plan` | OPT.T0.2 — Write optimization tasklist and publish roadmap |
| [#78](https://github.com/akai-hana/shiro-layout/issues/78) | `opt-generator` | OPT.T1.1 — Replace constrained secondary full rescoring with dual-delta scoring |
| [#79](https://github.com/akai-hana/shiro-layout/issues/79) | `opt-generator` | OPT.T1.2 — Parallelize ILS/polish with deterministic per-candidate RNG |
| [#80](https://github.com/akai-hana/shiro-layout/issues/80) | `opt-generator` | OPT.T1.3 — Micro-optimize delta scoring |
| [#81](https://github.com/akai-hana/shiro-layout/issues/81) | `opt-generator` | OPT.T1.4 — Record generator benchmarks and parity |
| [#82](https://github.com/akai-hana/shiro-layout/issues/82) | `opt-analyzer` | OPT.T2.1 — Parallelize table build and trim penalty-mask allocation |
| [#83](https://github.com/akai-hana/shiro-layout/issues/83) | `opt-analyzer` | OPT.T2.2 — Run report passes concurrently and trim fingering-guide churn |
| [#84](https://github.com/akai-hana/shiro-layout/issues/84) | `opt-analyzer` | OPT.T2.3 — Parallelize corpus loading and use numeric iteration |
| [#85](https://github.com/akai-hana/shiro-layout/issues/85) | `opt-analyzer` | OPT.T2.4 — Record analyzer benchmarks and parity |
| [#86](https://github.com/akai-hana/shiro-layout/issues/86) | `opt-prune` | OPT.T3.1 — Audit and prototype aggressive pruning |
| [#87](https://github.com/akai-hana/shiro-layout/issues/87) | `opt-prune` | OPT.T3.2 — Decide, gate, and document pruning |
| [#88](https://github.com/akai-hana/shiro-layout/issues/88) | `opt-verify` | OPT.T4.1 — Run final gates and benchmark verification |
| [#89](https://github.com/akai-hana/shiro-layout/issues/89) | `opt-verify` | OPT.T4.2 — Record completion, tag, and publish optimization branches |

Branch rule: one task equals one commit. Do not rewrite or edit
`RUST_REWRITE_SPEC_LIST.txt`. Do not include unrelated corpus edits.

## 4. Quality gates

Run from the repository root after every branch:

```bash
cargo fmt --manifest-path shirolyzer/Cargo.toml --check
cargo clippy --manifest-path shirolyzer/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path shirolyzer/Cargo.toml --release
shirolyzer/target/release/shirolyzer check-baseline
shirolyzer/target/release/shirolyzer analyze ";ntgfjoehkrpmscuaydilbqvwxz,./" --out /tmp/opt-analysis.txt --out-json /tmp/opt-analysis.json
shirolyzer/target/release/shirolyzer compare
shirolyzer/target/release/shirolyzer audit --out /tmp/opt-audit.txt --out-json /tmp/opt-audit.json
```

Parity gates:

1. **Numeric:** reference scores within `1e-9`, analyze primary
   `3.3940533330472595`, audit `166/166`, nine references.
2. **Incremental:** randomized swap/cycle and constrained dual-delta scores
   equal full rescoring within `1e-12`.
3. **Selection:** the small benchmark compute run selects
   `;ntgfjoehkrpmscuaydilbqvwxz,./` with primary `3.520201897371` and secondary
   `3.704852567378`.
4. **Determinism:** for a fixed seed set, generator JSON is identical with
   `RAYON_NUM_THREADS=1` and `RAYON_NUM_THREADS=16`.
5. **Analyzer:** text output is identical to the baseline; JSON scores,
   categories, usage, and guide counts are identical.

Performance gates:

```bash
cargo bench --manifest-path shirolyzer/Cargo.toml
```

Record wall clock, operations/second, before/after ratios, and score
equivalence in the phase report under `shirolyzer/results/rust/`.

## 5. Tasks

### Branch `opt-bench`

- [x] **OPT.T0.1 — Freeze optimization baseline and profiling evidence**
  - Files: `shirolyzer/results/rust/OPT_BASELINE.md`.
  - Record environment, micro-benchmarks, short-command medians, the small
    compute run, and the generator/analyzer profiles.
  - Done when the report is committed on `opt-bench`.

### Branch `opt-plan`

- [x] **OPT.T0.2 — Write optimization tasklist and publish roadmap**
  - Files: `RUST_OPTIMIZATION_TASKLIST.md`.
  - Publish milestone `Rust optimization`, labels, issues #76-#89, and the
    branch mapping above.
  - Done when the plan is committed and all issues are visible on GitHub.

### Branch `opt-generator`

- [x] **OPT.T1.1 — Replace constrained secondary full rescoring with dual-delta scoring**
  - Add a constrained quench that tracks primary and secondary weighted sums
    with two `Local` states, using the same move lists and shuffle order as
    `quench_generic`.
  - Keep `quench_generic` as the reference path and add a randomized
    dual-delta-vs-full test.
  - Done when the constrained optimum matches `quench_generic` within `1e-12`
    on benchmark seeds and the secondary phase is materially faster.

- [x] **OPT.T1.2 — Parallelize ILS/polish with deterministic per-candidate RNG**
  - Run independent ILS and polish candidates through rayon with RNG streams
    derived from the master seed, phase, row, and candidate index.
  - Preserve result ordering and printing; keep the time-budget path serial or
    budget-aware.
  - Done when a fixed seed produces identical JSON for 1 and 16 threads and
    quality is at least as good as the serial path on the benchmark set.

- [x] **OPT.T1.3 — Micro-optimize delta scoring**
  - Store key ids as `u8`, copy smaller arrays, replace the cross-table
    `HashMap<[u8; 4], Vec<u16>>` with a dense move-indexed table, and reuse
    scratch buffers where safe.
  - Keep the full-score fallback and randomized delta-vs-full tests.
  - Done when `delta_cross`/`delta` benchmarks improve and scores are unchanged.

- [x] **OPT.T1.4 — Record generator benchmarks and parity**
  - Add generator-focused bench output if needed and write
    `shirolyzer/results/rust/OPT_GENERATOR.md`.
  - Done when before/after numbers, determinism checks, and score/selection
    parity are recorded.

### Branch `opt-analyzer`

- [x] **OPT.T2.1 — Parallelize table build and trim penalty-mask allocation**
  - Build independent bigram/trigram table rows in parallel and reduce
    temporary allocations in `Gates::penalty_mask`, `forbidden_pairs`, and
    `Model::tri_many` without changing table values.
  - Done when table values still match the fixtures within `1e-12` and
    `Tables::build` is faster.

- [x] **OPT.T2.2 — Run report passes concurrently and trim fingering-guide churn**
  - Run category, trigram-pattern, finger-usage, and fingering-guide passes
    concurrently.
  - Encode trigram contexts without building a `String` per corpus entry; only
    format the surviving top entries.
  - Done when analyze text output is identical and JSON values/counts match.

- [x] **OPT.T2.3 — Parallelize corpus loading and use numeric iteration**
  - Read the three parsed JSON files concurrently, and use `bi_ids`/`tri_ids`
    with letter-index lookups instead of `HashMap<char, _>` lookups in the hot
    report loops.
  - Done when corpus totals, scores, categories, and guide output match.

- [x] **OPT.T2.4 — Record analyzer benchmarks and parity**
  - Write `shirolyzer/results/rust/OPT_ANALYZER.md` with before/after medians,
    output-diff evidence, and the remaining profile.
  - Done when all analyzer gates pass.

### Branch `opt-prune`

- [x] **OPT.T3.1 — Audit and prototype aggressive pruning**
  - Identify candidates: empty/short cross lists, dominated moves, repeated
    no-op deltas, rare-trigram pruning, bounded move subsets, and early
    quench exits.
  - Prototype only the options that can preserve the objective, behind a flag
    or separate function, and measure quality against the benchmark seeds.
  - Done when each option has a measured speed/quality result.

- [x] **OPT.T3.2 — Decide, gate, and document pruning**
  - Ship only pruning that preserves the selected layout and scores; otherwise
    leave it disabled or remove it.
  - Write `shirolyzer/results/rust/OPT_PRUNING.md`.
  - Done when the decision and any new flag/default are documented and gated.

### Branch `opt-verify`

- [x] **OPT.T4.1 — Run final gates and benchmark verification**
  - Run the full quality, parity, determinism, and performance gates from the
    final layout.
  - Compare with `OPT_BASELINE.md` and record the final numbers.
  - Done when all gates pass and the verification report is committed.

- [ ] **OPT.T4.2 — Record completion, tag, and publish optimization branches**
  - Write `shirolyzer/results/rust/OPT_SUMMARY.md`, mark this tasklist complete,
    and create annotated tag `rust-optimization`.
  - Push all six branches, close issues #76-#89, and fast-forward `main` to the
    final optimization commit.
  - Done when the epic is published and `main` contains the optimized Rust
    runtime.

## 6. Acceptance criteria

1. **Functional:** all existing tests pass, `check-baseline` is 166/166 with
   nine references, and analyze/compare/audit values are unchanged.
2. **Objective:** dual-delta and any pruning preserve scores within `1e-12` on
   randomized layouts and the benchmark seed set.
3. **Generator speed:** small compute improves by at least 1.5x; default
   compute shows a material wall-clock reduction.
4. **Analyzer speed:** `analyze` improves by at least 1.3x.
5. **Determinism:** fixed seeds produce identical JSON for 1 and 16 threads.
6. **Documentation:** every new flag, default, and pruning decision is recorded
   in the phase reports and this tasklist.

## 7. Progress log

- 2026-10-30: Froze the pre-optimization baseline, micro-benchmarks, small
  compute run, and perf profiles in `OPT_BASELINE.md`.
- 2026-10-30: Wrote this plan, created milestone `Rust optimization`, labels, and
  issues #76-#89, and published the roadmap.
- 2026-10-30: Replaced constrained secondary full rescoring with a dual-delta
  quench; the randomized parity test matches `quench_generic` exactly.
- 2026-10-30: Parallelized ILS rows and polish candidates with per-candidate RNG
  streams. Fixed seeds produce identical result JSON for 1 and 16 threads
  (apart from elapsed time and the output path).
- 2026-10-30: Stored key/trigram ids as `u8` and replaced the cross-table
  `HashMap` with a dense move-indexed `Vec`. Benchmarks improved from
  ~15.8 us to ~12.0 us per delta swap, ~221 ms to ~172 ms per quench, and
  ~61k to ~88k anneal iterations/s. Small compute dropped from ~2.27 s to
  ~1.73 s; selected layout and scores are unchanged.
- 2026-10-30: Recorded the generator before/after numbers, determinism check,
  and parity evidence in `shirolyzer/results/rust/OPT_GENERATOR.md`.
- 2026-10-30: Parallelized bigram/trigram table rows and added reusable
  `penalty_mask_into`/`tri_many_into` paths. `Tables::build` dropped from
  ~17.7 ms to ~9.6 ms with four Rayon threads; table fixtures and audit still
  pass within tolerance.
- 2026-10-30: Ran the category, trigram-pattern, finger-usage, and
  fingering-guide passes concurrently with `rayon::join`, and stored guide
  contexts as `[char; 3]` instead of formatting a `String` per corpus entry.
  Analyze text and JSON are unchanged; pinned four-thread median dropped from
  0.0346 s to 0.0305 s (1.13x).
- 2026-10-30: Read the three parsed corpus files concurrently and switched the
  report loops to the numeric `bi_ids`/`tri_ids` vectors with a fixed
  letter-index keymap. Corpus totals, scores, categories, and guide output are
  unchanged; the pinned median stayed flat at 0.0314 s (within run noise).
- 2026-10-30: Recorded analyzer before/after timings, output diffs, and the
  remaining profile in `shirolyzer/results/rust/OPT_ANALYZER.md`. Interleaved
  unpinned analyze medians improved from 0.0433 s to 0.0262 s (1.65x); text and
  JSON output are identical, and the remaining cost is now `Tables::build`.
- 2026-10-30: Audited and prototyped pruning candidates behind temporary
  environment hooks. All 44,850 two-swap cross lists are non-empty (minimum
  195 entries); bounded subsets, phase skips, and rare-trigram thresholds all
  changed the search path or hard-set quality. Recorded the measurements in
  `shirolyzer/results/rust/OPT_PRUNING.md`.
- 2026-10-30: Decided to ship no pruning: the full move set and defaults stay
  unchanged, and no flag or environment hook remains. Added
  `delta_quench_matches_full_quench`, which requires the delta quench and the
  full-score quench to return the same permutation and score for a fixed seed,
  as the regression gate for the decision.
- 2026-10-30: Ran the final gates on `opt-verify`: fmt, clippy, release tests,
  check-baseline, analyze, compare, audit, thread-count determinism, and the
  full benchmark suite all pass. Small compute is 2.01x faster than baseline
  and analyze is 1.60x faster; the default run selects the same shiro layout in
  30.6 s. Recorded the numbers in
  `shirolyzer/results/rust/OPT_VERIFICATION.md`.
