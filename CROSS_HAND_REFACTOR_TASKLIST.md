# Cross-hand (8-finger) model refactor tasklist

**Status: implemented and verified on `cross-hand/geometry` (working tree).**
The small rules (ring-pinky reverse-scissor block, `XA` middle/ring block, `GSW`
alt) were reviewed first. The remaining workflow steps are committing the phase
work, merging `cross-hand/refactor`, and closing the tracking issue.

## Objective

Teach the model to treat a key as typeable by either hand, so cross-hand
alternate fingerings such as H = left index (`LI`) and Y = left middle (`LM`)
are legal when explicitly requested by an `alt_fingers` rule. Cross-hand travel
must be priced as extra finger travel; it must not be treated as a different
key hand.

## Decisions already made

- Extend the model from 4 to 8 finger states: `hand * 4 + finger`
  (`LP/LR/LM/LI/RI/RM/RR/RP`).
- Cross-hand fingerings are allowed **only through `alt_fingers`**. The default
  `fingermap` stays same-hand.
- Price cross-hand travel from **absolute** key and finger positions. A left
  index on H is a moderate cross-body stretch, not a hand-relative huge
  displacement.
- Same-hand pair terms must use the **finger's hand**, not the key's hand.
- Do not change the parameter file or refit the model unless verification shows
  it is unavoidable.

## Git workflow

1. Create the tracking issue:
   ```sh
   gh issue create \
     --title "Cross-hand (8-finger) alt-fingering model" \
     --body-file CROSS_HAND_REFACTOR_TASKLIST.md
   ```
2. Create the integration branch from `main`:
   ```sh
   git switch main
   git pull --ff-only
   git switch -c cross-hand/refactor
   ```
3. Create one short-lived branch per phase below, merge back into
   `cross-hand/refactor` with `--no-ff`, and use the suggested commit messages.
4. Tag the pre-refactor checkpoint:
   ```sh
   git tag cross-hand-pre-refactor
   ```
5. Keep `main` green. Do not merge `cross-hand/refactor` until every gate below
   passes.

## Phase 0 — Baseline and design capture

- [x] Create the issue and branches listed above.
- [x] Record current `cargo test --release`, `check-baseline`, and benchmark
      results in the issue as the pre-refactor baseline.
- [x] Decide the concrete 8-state token encoding and document it in
      `shirolyzer/config/config.yaml` comments and `shirolyzer/README.md`.
- [x] Confirm whether `NF` stays a compile-time constant or becomes a derived
      constant; prefer compile-time `NF = 8` with named helpers.

Suggested commit: `cross-hand: capture pre-refactor baseline`

## Phase 1 — Geometry and finger states

Branch: `cross-hand/geometry`

- [x] Add a `Finger`/`FingerHand` abstraction in `shirolyzer/src/geometry.rs`
      (`FINGERS8`, `finger_hand`, `finger_index8`, etc.).
- [x] Keep the existing same-hand `du`/`dv`/`reach` tables for same-hand
      fingers.
- [x] Add absolute key positions and absolute rest positions for cross-hand
      travel. Do not reuse the mirrored hand-relative `u` for cross-hand.
- [x] Define cross-hand reach: allow it for explicitly permitted alt-rule
      fingerings, but keep the default reach table same-hand only.
- [x] Add geometry tests for H = LI / Y = LM travel and for same-hand
      regressions.
- [x] Verify no current geometry-dependent test changes unless intentional.

Suggested commit: `cross-hand: add 8-finger geometry and absolute travel`

## Phase 2 — Config and alt-rule syntax

Branch: `cross-hand/config`

- [x] Change `Config::fingermap` from a bare finger letter to an 8-state finger
      index, while validating that the shipped default map remains same-hand.
- [x] Change `AltRule::fingers` to 8-state indices.
- [x] Relax hand validation only for `alt_fingers`; keep it for `fingermap`.
- [x] Update the YAML parser and error messages in `shirolyzer/src/config.rs`.
- [x] Mirror the same handling in `shirolyzer/utils/config.py` for parity.
- [x] Update `shirolyzer/README.md` and the config comments with cross-hand
      examples (for example H = `LI`, Y = `LM`).
- [x] Add parser tests: same-hand default accepted, cross-hand alt accepted,
      invalid tokens rejected.

Suggested commit: `cross-hand: accept explicit cross-hand alt tokens`

## Phase 3 — Model and pair features

Branch: `cross-hand/model`

- [x] Raise `NF` to 8 and audit every `NF`/finger-index assumption.
- [x] Make `pair_feature` use the finger's hand for `same_hand`,
      `rest_sep`, and inner/outer separation.
- [x] Keep cross-hand pairs as hand alternation for the alternation term, but
      price the cross-hand key's travel separately.
- [x] Rebuild pair/trigram cost tables and check size/performance.
- [x] Add model tests for cross-hand pairs and same-hand regressions.
- [x] Do not change fitted parameter values; only the state mapping and
      cross-hand travel definition.

Suggested commit: `cross-hand: extend comfort model to eight finger states`

## Phase 4 — Scorer and tables

Branch: `cross-hand/scorer`

- [x] Update `shirolyzer/src/scorer.rs` `big_f`/`tri_f` encodings to 8-state
      indices.
- [x] Update `Tables::build` and all delta tables used by `search`.
- [x] Update `shirolyzer/tests/parity.rs` expected values only after the
      intended behavior is verified.
- [x] Benchmark `Tables::build` and scoring; cross-hand must not cause an
      unacceptable regression.

Suggested commit: `cross-hand: encode eight-state fingerings in scorer tables`

## Phase 5 — Analyzer, audit, and display

Branch: `cross-hand/analyzer`

- [x] Update `shirolyzer/src/analyze.rs` to use the finger's hand for usage,
      SFB/SFT classification, and hand balance.
- [x] Display selected fingerings with the full hand/finger token so H = LI is
      visible and not reported as H = RI.
- [x] Update hardcoded audit probes that assume key hand == finger hand.
- [x] Add rulings/probes for the user's cross-hand cases (H = LI, Y = LM) and
      for same-hand fallbacks.
- [x] Confirm the ring-pinky condition still applies only to same-hand pairs.

Suggested commit: `cross-hand: report and audit cross-hand fingerings`

## Phase 6 — Conditions, gates, and search

Branch: `cross-hand/rules`

- [x] Update `conditions.rs` pair masks and `gates.rs` penalties for 8 states.
- [x] Ensure `alt_fingers` matching is still by physical key sequence, with
      cross-hand tokens only where explicitly allowed.
- [x] Update `search.rs`/`compute.rs` delta tables and hand-restriction logic.
- [x] Keep default generation same-hand; cross-hand must only appear when an
      alt rule licenses it.
- [x] Run the full `just check` suite and the compute smoke test.

Suggested commit: `cross-hand: integrate rules and search with eight states`

## Phase 7 — Verification and documentation

Branch: `cross-hand/verify`

- [x] `cargo test --release --manifest-path shirolyzer/Cargo.toml`
- [x] `cargo fmt --manifest-path shirolyzer/Cargo.toml --check`
- [x] `cargo clippy --manifest-path shirolyzer/Cargo.toml --all-targets -- -D warnings`
- [x] `shirolyzer/target/release/shirolyzer check-baseline`
- [x] `just dev-tools`
- [x] Update `score_baseline.json` only for intentional score changes and
      document each one in the issue.
- [x] Add a short "cross-hand alt-fingering" section to `shirolyzer/README.md`.
- [ ] Close the issue with the final benchmark and test summary (pending commit
      and merge of `cross-hand/refactor`).

Suggested commit: `cross-hand: verify, baseline, and document the refactor`

### Verification results (post-refactor)

- `cargo test --release`: 19 unit + 1 baseline + 5 command + 3 parity passed;
  the ignored `compute_smoke` also passed when run explicitly.
- `shirolyzer check-baseline`: `baseline OK: audit 166/166, 9 references`.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`: clean.
- `just dev-tools`: clean.
- `score_baseline.json`: unchanged; same-hand reference scores and all 166
  audit rulings still pass.
- `cargo bench --bench score` (pre → post): tables build 13.326 → 59.402 ms
  (expected one-time cost of 8× more finger states); delta-table build
  54.841 → 59.425 ms; full score 18.01 → 17.43 µs/op; delta swap
  9634 → 9638 ns/op; quench 249.117 → 243.836 ms; full-score quench
  1545.438 → 1522.555 ms; anneal 97.796 → 97.557 ms per 10k iters.

## Acceptance criteria

- [x] `alt_fingers` can explicitly request H = `LI` and Y = `LM`, and the model
      accepts those fingerings with an absolute cross-body travel cost.
- [x] The default `fingermap` never produces a cross-hand fingering.
- [x] Same-hand pair terms use the finger's hand, not the key's hand.
- [x] Same-hand behavior and rulings remain unchanged except for explicitly
      documented rule changes.
- [x] `cargo test`, `check-baseline`, `fmt`, `clippy`, and `just dev-tools` pass.
- [x] The tracking issue records the pre/post benchmarks and any baseline
      updates.

## Relevant files

- `shirolyzer/src/geometry.rs` — `NF`, rest/reach, `du`/`dv`, hand assumptions.
- `shirolyzer/src/model.rs` — `pair_feature`, pair/trigram costs.
- `shirolyzer/src/scorer.rs` — `Tables`, `big_f`/`tri_f`, deltas.
- `shirolyzer/src/config.rs` — `fingermap`, `alt_fingers`, validation.
- `shirolyzer/src/gates.rs` — alt-rule matching and penalties.
- `shirolyzer/src/conditions.rs` — pair masks and geometric rules.
- `shirolyzer/src/analyze.rs` — hand/finger usage and SFB/SFT classification.
- `shirolyzer/src/audit.rs` — probes, rulings, and hard checks.
- `shirolyzer/src/search.rs`, `shirolyzer/src/compute.rs` — state space and
  delta tables.
- `shirolyzer/tests/{baseline,commands,parity}.rs` — regression gates.
- `shirolyzer/utils/` — Python development tooling and parity.
