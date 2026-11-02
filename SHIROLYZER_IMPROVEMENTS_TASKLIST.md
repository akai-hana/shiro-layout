# Shirolyzer analyzer improvements tasklist

**Status: phase 1 complete.** Improvements 1 and 2 are implemented and merged
to `main`. Improvements 3–8 are explicitly deferred so the phase-1 integration
can land; they can be tracked in a follow-up issue when prioritized.

Tracking issue: https://github.com/akai-hana/shiro-layout/issues/92

## Objective

Apply the localized analyzer improvements proposed after the Oxeylyzer
comparison without replacing Shiro's trigram-DP comfort model. The work is
split so each item can be reviewed, tested, and reverted independently.

## Decisions

- Keep Shiro's trigram-DP + fitted comfort model as the scoring source of
  truth. Oxeylyzer's integer weighted sum, full 14-class taxonomy,
  rectangle-gap geometry, and skipgram-decay score are not imported.
- A **true SFT** is a trigram whose three chosen fingers are all the same.
- An **SFB trigram** is a trigram with an adjacent same-finger pair that is
  not an SFT. Oxeylyzer's `Sfb` and `BadSfb` are deliberately lumped: whether
  all three keys are on one hand does not change the fact that an SFB exists.
- The combined SFB/SFT budget is
  `same_finger_bigram + sfb_trigram + 2 * sft_trigram`.
  Sfb/BadSfb therefore cost 1x and a true SFT costs 2x.
- `repeat` becomes an overlapping diagnostic for repeated characters. It no
  longer hides same-finger trigrams from the SFT/SFB taxonomy or the budget.
- Category-level diagnostics and budget accounting may change; the primary
  fitted score is untouched unless a later, separately gated experiment
  proves a change is necessary.

## Git workflow

1. Tracking issue:
   https://github.com/akai-hana/shiro-layout/issues/92
2. The integration branch `analyzer/improvements` started from
   `fix/alt-finger-positional-overlap` and was merged to `main` after the #91
   fix landed:
   ```sh
   git switch -c analyzer/improvements
   ```
3. Give each improvement a short-lived branch, merge it back with `--no-ff`,
   and use the suggested commit messages below.
4. Keep `main` green. Do not merge the integration branch until every gate in
   the verification section passes.
5. Update this tasklist in the same commit that completes each task so the
   checkboxes always describe the branch state.

Commit convention: `analyzer: <imperative summary>` with the issue reference in
the body, for example `analyzer: split SFT from SFB accounting (Refs #<issue>)`.

## Improvement 1 — Split SFT from SFB in diagnostics

Branch: `analyzer/improvements` (first phase; later phases use their own
branches).

- [x] Add a shared trigram classifier used by both `analyze` and `worst`.
- [x] Report `sft` for all-three-same-finger trigrams.
- [x] Report `sfb_tri` for adjacent same-finger trigrams that are not SFT.
- [x] Report `sfb_sft` as the combined `sft + sfb_tri` diagnostic.
- [x] Keep `repeat` visible as an overlapping diagnostic instead of an
      exclusive bucket.
- [x] Add unit tests for SFT, lumped Sfb/BadSfb, alternate, and non-SFT cases.
- [x] Add a parity regression test for exclusive categories, overlapping
      repeats, and the `sfb_sft` sum.
- [x] Document the new category semantics in `shirolyzer/README.md`.

Suggested commit: `analyzer: split SFT from adjacent SFB diagnostics`

## Improvement 2 — Fix repeat/SFB accounting in the sacrifice budget

Branch: `analyzer/improvements` (same first-phase branch).

- [x] Change the budget from `sfb + 2 * sfb_tri` to
      `sfb + sfb_tri + 2 * sft`.
- [x] Count repeated-character trigrams in their SFT/SFB class as well as in
      the `repeat` diagnostic.
- [x] Update the sacrifice comment in `config.yaml` and `config.fixed.yaml`.
- [x] Update the `compute_smoke` expected best-primary value for the
      intentional selection change.
- [x] Re-run the unit, command, parity, baseline, and ignored compute gates.

Suggested commit: `analyzer: weight true SFT twice an adjacent SFB`

## Deferred work

Improvements 3–8 below are intentionally deferred. Phase 1 delivers the
SFT/SFB taxonomy and budget fix from the original request; the remaining
observability and comparison work should be tracked separately to keep the
integration reviewable.

## Improvement 3 — Corpus provenance

Branch: `analyzer/corpus-provenance`

- [ ] Record the parser rule, source hash/version, and normalization in
      analysis output.
- [ ] Optionally allow ingesting an external ngram JSON for comparison runs
      only; keep Shiro's native parser and native score.
- [ ] Add a provenance snapshot test that does not depend on network access.

Suggested commit: `analyzer: record corpus provenance in reports`

## Improvement 4 — External-comparison mapping and warnings

Branch: `analyzer/comparison-mapping`

- [ ] Map Shiro categories to the community names (`Sfb`, `BadSfb`, `Sft`,
      `SFS`, and so on).
- [ ] Warn when a comparison is invalid: bigram vs trigram SFB, fixed vs
      flexible fingering, or different denominators.
- [ ] Keep the mapping in documentation/observability, not in the score.

Suggested commit: `analyzer: add external comparison mapping`

## Improvement 5 — Decompose Shiro's score

Branch: `analyzer/score-decomposition`

- [ ] Expose per-term contributions of Shiro's own model (pair SFB, scissor,
      redirect, roll, repeat, balance).
- [ ] Add the decomposition to `analyze --out-json` and a concise text table.
- [ ] Add a regression test that the sum of contributions reconstructs the
      reported primary score.

Suggested commit: `analyzer: expose score decomposition`

## Improvement 6 — Naming and semantics cleanup

Branch: `analyzer/naming-semantics`

- [ ] Align `W_REDIRECT_WEAK` (model: pinky involved) with
      `redirect_weak` (analysis: no index).
- [ ] Audit every `sfb_tri`/`repeat` use after the SFT split and rename any
      remaining ambiguous identifiers.
- [ ] Document the final taxonomy in one place and link to it from the README.

Suggested commit: `analyzer: align redirect and SFB naming`

## Improvement 7 — Geometry known-answer tests and debug dump

Branch: `analyzer/geometry-tests`

- [ ] Add known-answer tests for physical pair distances on the shipped board.
- [ ] Add a debug dump mode for geometry and reachability used by tests only.
- [ ] Compare Shiro distances directly instead of porting Oxeylyzer geometry.

Suggested commit: `analyzer: add geometry known-answer tests`

## Improvement 8 — Optional report-only extras

Branch: `analyzer/report-extras`

- [ ] Add a skipgram/SFS distance ladder as a report-only metric.
- [ ] Add a shift/punctuation-layer audit.
- [ ] Add either metric to the score only if a controlled experiment shows it
      changes Shiro's own optimum.

Suggested commit: `analyzer: add optional skipgram and shift reports`

## Verification

- [x] `cargo fmt --manifest-path shirolyzer/Cargo.toml --check`
- [x] `cargo clippy --manifest-path shirolyzer/Cargo.toml --all-targets -- -D warnings`
- [x] `cargo test --manifest-path shirolyzer/Cargo.toml --release`
- [x] `cargo test --manifest-path shirolyzer/Cargo.toml --release --test commands compute_smoke -- --ignored`
- [x] Baseline and audit gates via the reference config
      (`cargo test --release --test commands audit_and_check_baseline_pass`)
- [ ] `shirolyzer/target/release/shirolyzer check-baseline` against the shipped
      `config.yaml` (blocked by the pre-existing shipped-config vs
      reference-baseline mismatch documented in
      `research/plans/alt-finger-positional-overlap.md`; the reference-config
      gate passes)
- [x] `just dev-tools`
- [x] Re-run the full gate suite on `analyzer/improvements` before merging to
      `main`.

## Acceptance criteria

- [x] `analyze` distinguishes true SFT from adjacent SFB trigrams.
- [x] Sfb and BadSfb are lumped in the analyzer output and budget.
- [x] A true SFT contributes 2x an adjacent SFB to the sacrifice budget.
- [x] Repeated-character trigrams remain visible through `repeat` without
      hiding their SFT/SFB class.
- [x] The remaining improvements are explicitly deferred with a written
      decision.
- [x] The tracking issue records the final gates and benchmark/score changes.

## Relevant files

- `shirolyzer/src/analyze.rs` — trigram classifier, category reports, output.
- `shirolyzer/src/worst.rs` — shared trigram classification for worst ngrams.
- `shirolyzer/src/compute.rs` — primary-sacrifice metrics and budget.
- `shirolyzer/config/config.yaml`, `config.fixed.yaml` — sacrifice comments.
- `shirolyzer/tests/commands.rs` — command and compute-smoke expectations.
- `shirolyzer/README.md` — user-facing category and budget semantics.
- `shirolyzer/src/scorer.rs`, `evaluate.rs` — later score decomposition work.
- `shirolyzer/src/geometry.rs` — later geometry known-answer tests.
