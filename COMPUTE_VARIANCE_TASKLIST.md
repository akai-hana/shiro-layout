# Compute run-to-run variance tasklist

**Status: in progress.** This plan turns
`Reducing run-to-run variance in shirolyzer compute.md` into reviewable,
separately testable changes. Each phase has its own branch, commit, and gate.
The final phase merges every branch into `main` and removes all temporary
branches.

Tracking issue: https://github.com/akai-hana/shiro-layout/issues/95

## Objective

Make two consecutive `shirolyzer compute` runs agree much more often, by
fixing the incremental/full score mismatch first, then improving breadth,
diversity, and stopping rules, and finally selecting a budget from measured
variance rather than guesswork.

## Decisions

- **Correctness before tuning.** Do not tune restart counts until the three
  `search.rs` incremental-score tests pass and `final` agrees with `primary`.
- The search objective and the selection objective must be the same number.
  The incremental score must include the balance/hand penalty, not only the
  weighted trigram term.
- Keep random starts as the default. Reference/warm starts remain opt-in
  through `--seed-reference` and `--seed-all-references`.
- New stopping behaviour is opt-in until the variance measurement justifies
  changing defaults.
- Deduplicate ILS candidates by **letter placement**, not by punctuation
  placement. Punctuation is handled separately and never decides which letter
  arrangements advance to ILS.
- Do not change the fitted model or baseline scores. This work changes search
  bookkeeping and scheduling, not the objective.

## Git workflow

1. Create the tracking issue from this file:
   ```sh
   gh issue create \
     --title "Reduce run-to-run variance in shirolyzer compute" \
     --body-file COMPUTE_VARIANCE_TASKLIST.md
   ```
2. Create the integration branch from `main`:
   ```sh
   git switch main
   git pull --ff-only
   git switch -c compute/variance
   ```
3. For each phase, create a short-lived branch from `compute/variance`, make
   the changes, run that phase's gate, and merge it back with `--no-ff`.
4. Update this tasklist in the same commit that completes each phase.
5. Do not merge `compute/variance` into `main` until the full verification
   section passes.
6. At the end, merge `compute/variance` into `main`, delete every temporary
   branch, and verify that only `main` remains.

Commit convention: `compute: <imperative summary>` with the issue reference in
the body, for example `compute: match incremental balance penalty (Refs #<issue>)`.

## Phase 0 — Baseline capture

Branch: `compute/variance`

- [x] Create the tracking issue and integration branch.
- [x] Record the baseline in the issue:
      three failing `search.rs` tests, current default flags, and the three
      primary values quoted in the analysis document. The reference baseline
      test was already red on `main` after the `finger_balance` priority change
      in 280d11e; that failure is unrelated to this work.
- [x] Confirm `git status` contains no staged changes and leave unrelated
      working-tree changes alone.

Suggested commit: `docs: plan compute variance reduction`

## Phase 1 — Fix incremental score drift

Branch: `compute/score-drift`

Problem: `Local::new` computes `pen = ev.penalty(perm)`, but `swap`,
`cycle3`, and their undo helpers never update it. The annealer therefore
optimizes a stale penalty, and `final` diverges from `primary`.

Steps:

- [x] Add a penalty-aware update to `Local::swap` and `Local::cycle3` so
      `score()` always equals `ev.score(current_perm)`.
- [x] Restore the matching penalty state in `undo_swap` and `undo_cycle3`
      without changing their public call sites.
- [x] Add a cheap `Local::penalty_after(swaps)` helper for candidate moves.
- [x] Include the penalty delta in the two-pair prediction used by `quench`
      and `quench_constrained`.
- [x] Update `incremental_scores_match_full_score` so its two-pair prediction
      explicitly includes the penalty delta.
- [x] Run `cargo test --release --lib search::tests` and confirm all three
      previously failing tests pass.
- [x] Run the full `cargo test --release` and `compute_smoke` gate.

Suggested commit: `compute: match incremental balance penalty`

## Phase 2 — Diverse ILS candidates and punctuation separation

Branch: `compute/ils-diversity`

Steps:

- [x] Add `--ils-candidates` (default `4`) to `ComputeArgs` and replace the
      hardcoded `top.truncate(4)`.
- [x] Add a letter-only layout distance helper in `diversity.rs`
      (differences in alphabetic placement only; punctuation differences do
      not count).
- [x] Select ILS candidates by letter-placement diversity with a minimum
      distance of `4`, filling from the ranked list only if too few diverse
      rows exist.
- [x] Key the `hits` column and rediscovery checks by letter placement so that
      punctuation-only siblings no longer look like independent rediscoveries.
- [x] Add a punctuation canonicalization step for the final selected layout so
      punctuation order is deterministic and does not create spurious unique
      layouts. Scoring is unchanged.
- [x] Keep the top-of-list ordering stable by primary, then final score.
- [x] Add unit tests for `letter_distance`, diverse selection, and
      punctuation canonicalization.
- [x] Document the new flag and the punctuation rule in `shirolyzer/README.md`.

Suggested commit: `compute: diversify ILS by letter placement`

## Phase 3 — Rediscovery and stale-best stopping

Branch: `compute/rediscovery`

Steps:

- [x] Add `--stop-hits` (default `0`, disabled), `--stop-stale-batches`
      (default `0`, disabled), and `--batch-size` (default `8`) to the CLI.
- [x] Process random starts in deterministic batches, parallel inside a batch
      and sequential between batches.
- [x] Track the best primary and the number of batches since the last strict
      improvement.
- [x] Stop early when the best letter placement has been independently found
      by `--stop-hits` starts.
- [x] Stop early when the best primary has not improved for
      `--stop-stale-batches` consecutive batches.
- [x] Keep `--runs` as the hard cap and always run at least one batch.
- [x] Record the stop reason and completed batch count in the text and JSON
      output.
- [x] Add tests for the hit-count and stale-batch decisions using synthetic
      result rows.
- [x] Verify that the disabled defaults reproduce the current behaviour.

Suggested commit: `compute: stop after rediscovery or stagnation`

## Phase 4 — Variance harness and budget selection

Branch: `compute/variance-budget`

Steps:

- [ ] Add a small development helper (Python is acceptable because it is
      tooling only) that runs `compute` once per seed and reports the spread of
      `best_primary`.
- [ ] Add a `just variance` recipe that builds the release binary, runs the
      helper, and prints min/max/mean/standard deviation/success rate.
- [ ] Run the harness with the pre-change budget and a breadth-heavy budget
      (`--runs 200 --iters 25000`) after Phase 1 is merged.
- [ ] Compare the spread and pick the new documented defaults for `--runs`,
      `--iters`, and, if useful, `--stop-hits`/`--stop-stale-batches`.
- [ ] Record the measurement command, seed range, spread, and decision in the
      tracking issue and in `shirolyzer/README.md`.
- [ ] Do not change baselines or fitted scores as part of this phase.

Suggested commit: `compute: measure variance and choose defaults`

## Phase 5 — Verification, documentation, and merge

Branch: `compute/variance` (integration branch)

Steps:

- [ ] `cargo fmt --manifest-path shirolyzer/Cargo.toml --check`
- [ ] `cargo clippy --manifest-path shirolyzer/Cargo.toml --all-targets -- -D warnings`
- [ ] `cargo test --manifest-path shirolyzer/Cargo.toml --release`
- [ ] `cargo test --manifest-path shirolyzer/Cargo.toml --release --test commands compute_smoke -- --ignored`
- [ ] `just dev-tools`
- [ ] Run a short before/after `compute` comparison with a fixed seed and
      confirm `final` and `primary` agree.
- [ ] Update this tasklist, the README, and the tracking issue with the final
      gates and chosen defaults.
- [ ] Merge `compute/variance` into `main` with `--no-ff`.
- [ ] Close the tracking issue with the final verification summary.

Suggested commit: `compute: verify variance reduction and document defaults`

## Final git cleanup

- [ ] Delete merged phase branches:
      `compute/score-drift`, `compute/ils-diversity`,
      `compute/rediscovery`, `compute/variance-budget`.
- [ ] Delete the integration branch `compute/variance` after it is merged.
- [ ] Fetch and prune remote-tracking branches only if a remote branch was
      created for this work.
- [ ] Verify the repository ends on `main` with no other local branches:
      ```sh
      git switch main
      git branch
      git log --oneline --graph --decorate -15
      ```
- [ ] Confirm every phase commit is reachable from `main` and the working tree
      contains only intentional changes.
- [ ] Do not delete pre-existing tags or archives (`cross-hand-pre-refactor`,
      `archive/*`, `backup/*`, `rust-*`).

## Verification gates

- [ ] All three previously failing incremental-score tests pass.
- [ ] `final` and `primary` agree for the same layout in a normal run.
- [ ] ILS candidates differ by at least the configured letter distance when
      enough diverse rows exist.
- [ ] `--ils-candidates` changes the number of ILS seed layouts.
- [ ] `--stop-hits` and `--stop-stale-batches` stop only when their thresholds
      are met and are disabled by default.
- [ ] `--batch-size` does not change results when stopping is disabled.
- [ ] Variance measurement is recorded and the chosen defaults are justified.
- [ ] `fmt`, `clippy`, `test`, `compute_smoke`, and `just dev-tools` pass.
- [ ] Only `main` remains after cleanup.

## Acceptance criteria

- [ ] The annealer and the selection step optimize the same score.
- [ ] Restart breadth can be increased without changing the scoring model.
- [ ] ILS candidates are diverse by letter placement.
- [ ] Rediscovery/stagnation stopping is available and documented.
- [ ] The final `--runs`/`--iters` defaults come from measured variance.
- [ ] All work is merged into `main` and all temporary branches are deleted.

## Relevant files

- `shirolyzer/src/search.rs` — incremental scoring, penalty update, quench,
      ILS/anneal primitives, incremental-score tests.
- `shirolyzer/src/compute.rs` — start scheduling, stopping rules, ILS
      selection, hits, output fields.
- `shirolyzer/src/diversity.rs` — letter distance, diverse selection,
      result merging.
- `shirolyzer/src/evaluate.rs` — full score, penalty, layout string.
- `shirolyzer/src/main.rs` — new CLI flags.
- `shirolyzer/tests/commands.rs` — compute smoke expectations.
- `shirolyzer/README.md` — user-facing compute flags and variance guidance.
- `justfile` — variance helper recipe.
- `Reducing run-to-run variance in shirolyzer compute.md` — source analysis.
