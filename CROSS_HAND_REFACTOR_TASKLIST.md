# Cross-hand (8-finger) model refactor tasklist

**Status: deferred.** The small rules (ring-pinky reverse-scissor block, `XA`
middle/ring block, `GSW` alt) are implemented separately. Do **not** start this
refactor until the small-rule checkpoint is reviewed.

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

- [ ] Create the issue and branches listed above.
- [ ] Record current `cargo test --release`, `check-baseline`, and benchmark
      results in the issue as the pre-refactor baseline.
- [ ] Decide the concrete 8-state token encoding and document it in
      `shirolyzer/config/config.yaml` comments and `shirolyzer/README.md`.
- [ ] Confirm whether `NF` stays a compile-time constant or becomes a derived
      constant; prefer compile-time `NF = 8` with named helpers.

Suggested commit: `cross-hand: capture pre-refactor baseline`

## Phase 1 — Geometry and finger states

Branch: `cross-hand/geometry`

- [ ] Add a `Finger`/`FingerHand` abstraction in `shirolyzer/src/geometry.rs`
      (`FINGERS8`, `finger_hand`, `finger_index8`, etc.).
- [ ] Keep the existing same-hand `du`/`dv`/`reach` tables for same-hand
      fingers.
- [ ] Add absolute key positions and absolute rest positions for cross-hand
      travel. Do not reuse the mirrored hand-relative `u` for cross-hand.
- [ ] Define cross-hand reach: allow it for explicitly permitted alt-rule
      fingerings, but keep the default reach table same-hand only.
- [ ] Add geometry tests for H = LI / Y = LM travel and for same-hand
      regressions.
- [ ] Verify no current geometry-dependent test changes unless intentional.

Suggested commit: `cross-hand: add 8-finger geometry and absolute travel`

## Phase 2 — Config and alt-rule syntax

Branch: `cross-hand/config`

- [ ] Change `Config::fingermap` from a bare finger letter to an 8-state finger
      index, while validating that the shipped default map remains same-hand.
- [ ] Change `AltRule::fingers` to 8-state indices.
- [ ] Relax hand validation only for `alt_fingers`; keep it for `fingermap`.
- [ ] Update the YAML parser and error messages in `shirolyzer/src/config.rs`.
- [ ] Mirror the same handling in `shirolyzer/utils/config.py` for parity.
- [ ] Update `shirolyzer/README.md` and the config comments with cross-hand
      examples (for example H = `LI`, Y = `LM`).
- [ ] Add parser tests: same-hand default accepted, cross-hand alt accepted,
      invalid tokens rejected.

Suggested commit: `cross-hand: accept explicit cross-hand alt tokens`

## Phase 3 — Model and pair features

Branch: `cross-hand/model`

- [ ] Raise `NF` to 8 and audit every `NF`/finger-index assumption.
- [ ] Make `pair_feature` use the finger's hand for `same_hand`,
      `rest_sep`, and inner/outer separation.
- [ ] Keep cross-hand pairs as hand alternation for the alternation term, but
      price the cross-hand key's travel separately.
- [ ] Rebuild pair/trigram cost tables and check size/performance.
- [ ] Add model tests for cross-hand pairs and same-hand regressions.
- [ ] Do not change fitted parameter values; only the state mapping and
      cross-hand travel definition.

Suggested commit: `cross-hand: extend comfort model to eight finger states`

## Phase 4 — Scorer and tables

Branch: `cross-hand/scorer`

- [ ] Update `shirolyzer/src/scorer.rs` `big_f`/`tri_f` encodings to 8-state
      indices.
- [ ] Update `Tables::build` and all delta tables used by `search`.
- [ ] Update `shirolyzer/tests/parity.rs` expected values only after the
      intended behavior is verified.
- [ ] Benchmark `Tables::build` and scoring; cross-hand must not cause an
      unacceptable regression.

Suggested commit: `cross-hand: encode eight-state fingerings in scorer tables`

## Phase 5 — Analyzer, audit, and display

Branch: `cross-hand/analyzer`

- [ ] Update `shirolyzer/src/analyze.rs` to use the finger's hand for usage,
      SFB/SFT classification, and hand balance.
- [ ] Display selected fingerings with the full hand/finger token so H = LI is
      visible and not reported as H = RI.
- [ ] Update hardcoded audit probes that assume key hand == finger hand.
- [ ] Add rulings/probes for the user's cross-hand cases (H = LI, Y = LM) and
      for same-hand fallbacks.
- [ ] Confirm the ring-pinky condition still applies only to same-hand pairs.

Suggested commit: `cross-hand: report and audit cross-hand fingerings`

## Phase 6 — Conditions, gates, and search

Branch: `cross-hand/rules`

- [ ] Update `conditions.rs` pair masks and `gates.rs` penalties for 8 states.
- [ ] Ensure `alt_fingers` matching is still by physical key sequence, with
      cross-hand tokens only where explicitly allowed.
- [ ] Update `search.rs`/`compute.rs` delta tables and hand-restriction logic.
- [ ] Keep default generation same-hand; cross-hand must only appear when an
      alt rule licenses it.
- [ ] Run the full `just check` suite and the compute smoke test.

Suggested commit: `cross-hand: integrate rules and search with eight states`

## Phase 7 — Verification and documentation

Branch: `cross-hand/verify`

- [ ] `cargo test --release --manifest-path shirolyzer/Cargo.toml`
- [ ] `cargo fmt --manifest-path shirolyzer/Cargo.toml --check`
- [ ] `cargo clippy --manifest-path shirolyzer/Cargo.toml --all-targets -- -D warnings`
- [ ] `shirolyzer/target/release/shirolyzer check-baseline`
- [ ] `just dev-tools`
- [ ] Update `score_baseline.json` only for intentional score changes and
      document each one in the issue.
- [ ] Add a short "cross-hand alt-fingering" section to `shirolyzer/README.md`.
- [ ] Close the issue with the final benchmark and test summary.

Suggested commit: `cross-hand: verify, baseline, and document the refactor`

## Acceptance criteria

- [ ] `alt_fingers` can explicitly request H = `LI` and Y = `LM`, and the model
      accepts those fingerings with an absolute cross-body travel cost.
- [ ] The default `fingermap` never produces a cross-hand fingering.
- [ ] Same-hand pair terms use the finger's hand, not the key's hand.
- [ ] Same-hand behavior and rulings remain unchanged except for explicitly
      documented rule changes.
- [ ] `cargo test`, `check-baseline`, `fmt`, `clippy`, and `just dev-tools` pass.
- [ ] The tracking issue records the pre/post benchmarks and any baseline
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
