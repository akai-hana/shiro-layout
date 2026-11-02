# Redirect tier preferences plan

Issue: TBD (create with the tracking-issue section below)
Branch: `analyzer/redirect-tiers`

## Summary

Teach shirolyzer to treat same-hand redirects as a ranked set of finger
patterns instead of one binary "regular vs weak" category.  The user config
will declare ordered tiers of redirect finger sequences.  The analyzer will
report redirects per tier and per sequence, and the generator/model will price
higher tiers as more costly so tier 1 redirects are preferred and the last tier
is avoided most.

This is a preference layer on top of the existing calibrated model.  The fitted
geometry, corpora, gates, and alt-finger logic do not change.

## Current behavior

Three places currently encode redirect behavior:

- `shirolyzer/src/model.rs:632` detects a same-hand turn with
  `d1 * d2 < 0`.
- `shirolyzer/src/model.rs:645` charges `W_REDIRECT_WEAK` when any finger is a
  pinky and `W_REDIRECT` otherwise.  Both are multiplied by
  `Scales::redirect`.
- `shirolyzer/src/analyze.rs:192` (`trigram_pattern`) classifies a same-hand
  turn as `redirect` when an index is involved and `redirect_weak` otherwise.
  `analyze.rs:237` aggregates those into `total_redirects`.

The model and analyzer rules are related but not identical, and neither can
distinguish `1-2-1` from `4-2-4`.  `worst` also prints the same coarse label
(`shirolyzer/src/worst.rs:224`).

## Finger notation and mapping

The config uses per-hand finger ranks, not physical left/right direction:

| User number | Finger | Internal `finger_kind` |
|-------------|--------|------------------------|
| 1 | index  | 3 |
| 2 | middle | 2 |
| 3 | ring   | 1 |
| 4 | pinky  | 0 |

The model stores finger kinds as `P=0, R=1, M=2, I=3`; convert with
`rank = 4 - finger_kind(f)`.  The same mapping applies to both hands, so
`1-2-1` means index-middle-index on either hand.  There is no left/right
mirroring in the config notation.

Only same-hand trigrams are redirect candidates.  Cross-hand alternation,
rolls, `sfb_tri`, `sft`, one-hand non-turns, and every bigram rule stay as they
are.

## Proposed config schema

Add an optional top-level section.  Order is significant: the first inner list
is tier 1 (most preferred), the last inner list is the most avoided.

```yaml
# Redirect preferences, most preferred first, most avoided last.
# Fingers are numbered per hand: 1=index, 2=middle, 3=ring, 4=pinky.
# Each entry is a same-hand finger sequence; the first and third differ in
# direction from the middle (e.g. 1-2-1, 2-1-3).
redirect_tiers:
  - ["1-2-1", "2-1-2", "1-3-1", "3-1-3"]
  - ["3-2-3"]
  - ["2-1-3", "3-1-2", "2-3-2", "1-4-1"]
  - ["4-1-4", "4-2-4", "2-4-2"]
  - ["3-4-3", "4-3-4", "3-2-4", "4-2-3"]
```

Mirror the same section into `config/config.fixed.yaml`.  Fixed and flexible
mode should use the same preference curve; only fingering selection differs.

### Parsing and validation

Add `redirect_tiers: Option<Vec<Vec<[u8; 3]>>>` to `Config`
(`src/config.rs:131`) and a parser next to the other section parsers
(`parse_priority`, `parse_alt_fingers`, etc.).

Validation should reject, with the config path and exact location:

- a non-list section or an empty tier list;
- an empty tier (consistent with `priority_list`);
- an entry that is not exactly three `1`-`4` numbers separated by `-`;
- values outside `1..=4`;
- duplicates within a tier or across tiers;
- a sequence that is not a same-hand direction reversal in user rank order
  (for example `1-2-3` is linear and must fail);
- an all-same sequence such as `2-2-2` (that is `sft`, not a redirect).

Add `"redirect_tiers"` to the known top-level sections at
`src/config.rs:1024`.  Keep it optional so existing configs continue to load.

## Semantics

### Tier cost curve

`priority_list` is about how strongly an objective is weighted: a high-priority
objective gets a larger penalty.  Redirect tiers are a preference ordering, so
the direction is the opposite.  Do not reuse `priority_multipliers()` for
redirects.

Recommended curve for `N` configured tiers, tier index `t` from 0 to `N-1`:

```text
base_cost(tier t) = W_REDIRECT
                  + (W_REDIRECT_WEAK - W_REDIRECT) * t / max(1, N - 1)
```

With the five tiers above this gives:

| Tier | Base cost |
|------|-----------|
| 1 | 0.30 |
| 2 | 0.675 |
| 3 | 1.05 |
| 4 | 1.425 |
| 5 | 1.80 |

The final redirect cost is `base_cost * Scales::redirect`, preserving the
existing `priority_list` influence.  The endpoints reuse the calibrated
`W_REDIRECT` and `W_REDIRECT_WEAK` constants so the new scale stays in the same
numeric range as the old model.  If the user adds or removes tiers, the spacing
recomputes automatically; the displayed tier order is what matters.

An alternative is explicit per-tier numeric weights in YAML, but that is not
needed for this request and can be added later without changing the tier list
format.

### Unlisted redirects

The 16 listed sequences are not all 28 possible same-hand turns.  The 12
currently unlisted patterns are:

```text
1-3-2  1-4-2  1-4-3  2-1-4  2-3-1  2-4-1
2-4-3  3-1-4  3-4-1  3-4-2  4-1-2  4-1-3
```

Recommended fallback: any valid same-hand turn not named in `redirect_tiers`
keeps the legacy rule (pinky involved => `W_REDIRECT_WEAK`, otherwise
`W_REDIRECT`) times `Scales::redirect`.  This avoids assigning an arbitrary
tier to patterns the user did not classify and keeps old configs/scores stable
for those patterns.

Analyzer output should expose them as `redirect_unclassified` (and optionally
retain the legacy `redirect` / `redirect_weak` diagnostics for one release).
If strict coverage is wanted later, add an option such as
`redirect_unlisted: legacy | weakest | error`; do not make `error` the default
because it would break the shipped list immediately.

### Interaction with sacrifice and priority

- `priority_list` keeps its current meaning: the `redirects` objective still
  scales every redirect cost through `Scales::redirect`.
- `general.primary_sacrifice.redirects` remains the aggregate
  `total_redirects` budget.  Per-tier sacrifice budgets are a non-goal for this
  change; add them later only if secondary optimization needs tier-specific
  constraints.
- Calibration/audit should remain independent of user preferences.  Put the
  tier cost table in `Scales` (e.g. `Option<RedirectCostTable>`), whose
  `Default` is `None`.  `Context::calibration()` already resets `Scales`, so
  the audit keeps using the fitted legacy behavior while normal analyze and
  compute use the configured tiers.

## Implementation outline

### 1. Shared redirect classifier

Create one shared source of truth, either in a new `src/redirect.rs` module or
as part of `model`/`config` with public helpers.  It should provide:

- `rank(finger: usize) -> u8` (`4 - finger_kind(finger)`);
- `is_turn([rank; 3]) -> bool`;
- `pattern_for([finger; 3]) -> Option<[u8; 3]>` when all three fingers share a
  hand and form a turn;
- a parsed `RedirectTiers` table with `tier_of(pattern)` and `base_cost`, plus
  legacy fallback helpers.

Both the model and analyzer must use this module.  Do not duplicate the turn or
tier logic in `analyze.rs`.

### 2. Model and generator

- Extend `Scales` (`src/model.rs:171`) with an optional redirect cost table.
  Keep `Scales: Copy` if practical; a `[Option<f64>; 64]` or `[[[f64; 4]; 4]; 4]`
  table indexed by rank triple is copyable and avoids hashing in the hot loop.
- In `Model::tri_many_into` (`src/model.rs:608`), when
  `same && turn`, look up the tier base cost.  If the pattern is configured use
  that cost; otherwise keep the existing `has_p ? W_REDIRECT_WEAK :
  W_REDIRECT` fallback.  Then multiply by `self.scales.redirect`.
- Leave `W_CONTINUE`, scissors, SFS, and every non-turn branch unchanged.
- `config::build_model` (`src/config.rs:1221`) should install the parsed tier
  table into `model.scales`.
- Confirm the generator already consumes the model trigram table
  (`scorer::Tables`), so no search rewrite is needed.  Changing the table is
  enough for `compute`, ILS, polish, and diversity scoring.

### 3. Analyzer

- Replace the `&'static str` return from `trigram_pattern` with a small
  structured result carrying the base category, optional redirect pattern, and
  optional tier.  Callers that only need text can use a `Display`/label helper.
- In `trigram_patterns` (`src/analyze.rs:237`), pass in the redirect tier
  configuration.  For a same-hand turn:
  - emit `redirect_tier_<n>` totals for each configured tier;
  - emit `redirect_<a>_<b>_<c>` counts for each configured sequence;
  - emit `redirect_unclassified` for legacy fallback patterns;
  - keep `total_redirects` as the sum of every redirect.
- Keep the existing exclusive trigram categories (`alternate`, `inroll`,
  `outroll`, `onehand`, `sft`, `sfb_tri`, `other`) disjoint.  Tier and pattern
  keys are overlapping diagnostics, like the existing `repeat` key.
- Update the text report (`src/analyze.rs:548`):
  - print tier totals in tier order;
  - print the sequences inside each tier with percentages;
  - keep `total_redirects`;
  - mark `redirect` / `redirect_weak` deprecated if they are retained.
- Update `compute::Metrics::evaluate` (`src/compute.rs:60`) to pass the config
  into `trigram_patterns`.  `total_redirects` must continue to drive the
  primary-sacrifice metric.
- Update `worst` (`src/worst.rs:224`) so its `pattern` column shows
  `redirect_tier_3:2-1-3` (or similar) instead of only `redirect`.

### 4. JSON shape

Minimal change: add the new flat keys to the existing `trigram_patterns` map in
the analysis JSON.  This keeps existing JSON readers working.

If a structured form is useful, add it in addition, not instead:

```json
"redirect_tiers": [
  {
    "tier": 1,
    "total_pct": 1.23,
    "patterns": [{"pattern": "1-2-1", "pct": 0.42}]
  }
]
```

### 5. Python development tooling

The Rust binary is the runtime implementation.  If refitting/LOO remains an
actively used workflow, mirror the parser and cost lookup in
`shirolyzer/utils/config.py` and `shirolyzer/utils/comfort.py` in the same PR.
Otherwise explicitly note in the plan/PR that the Python tools still implement
the legacy curve and open a follow-up issue, so the divergence is intentional.

> Implementation status: the Rust runtime, config, analyzer, generator, and
> fixtures are implemented on this branch.  The retained Python fitting tools
> mirror the redirect tier parser and cost lookup, and fall back to the legacy
> curve when no tiers are configured.

## Tests

Add or update:

- Config parser tests:
  - the full shipped section parses and preserves tier order;
  - rank conversion maps `1-2-1` to internal kinds `[3, 2, 3]`;
  - valid patterns on the right hand classify identically to the left;
  - malformed tokens, out-of-range numbers, duplicates, linear sequences, and
    all-same sequences fail with useful messages;
  - no section => legacy behavior.
- Shared classifier unit tests:
  - every listed pattern is a turn and maps to the expected tier;
  - all 12 unlisted turns fall back to legacy classification;
  - `2-2-2`, `1-2-3`, and cross-hand sequences are not redirects.
- Model tests:
  - `base_cost` is strictly increasing by tier;
  - a configured pattern uses its tier cost and an unlisted turn keeps the
    legacy cost;
  - `Scales::default()` ignores the tier table (calibration path);
  - `Scales::redirect` multiplies both configured and fallback costs.
- Analyzer tests:
  - known finger triples produce the expected tier/pattern labels;
  - tier and pattern percentages sum consistently with `total_redirects`;
  - the exclusive non-redirect categories still sum to 100%;
  - the left/right versions of the same rank triple land in one tier.
- `worst` test:
  - the pattern column contains a tier label for a redirect ngram.
- Regression fixtures:
  - `shirolyzer/tests/parity.rs` tri-table known answers;
  - `shirolyzer/tests/baseline.rs` reference trigram scores;
  - `shirolyzer/tests/commands.rs` analyze score, compare output, compute smoke
    expectation;
  - `shirolyzer/config/score_baseline.json` via `check-baseline --write` only
    after confirming every changed number is explained by the new redirect
    prices.

Run:

```bash
just build
just test
just check
cargo test --manifest-path shirolyzer/Cargo.toml --release -- --ignored
```

(`just dev-tools` too if the Python tooling is updated.)

## Migration and compatibility

- `redirect_tiers` absent: exact legacy behavior, including legacy analyzer
  labels.
- `redirect_tiers` present: configured sequences use tier costs; unlisted
  turns use legacy fallback costs.
- Keep `total_redirects` stable in meaning so `primary_sacrifice.redirects`
  continues to work.
- Prefer keeping `redirect` / `redirect_weak` for one release as deprecated
  overlapping diagnostics.  Remove them in a follow-up after downstream users
  have moved to `redirect_tier_*` and `redirect_*_*_*`.
- `config.fixed.yaml` must remain score-compatible with `config.yaml` apart from
  fingering mode.  Both ship the same tier list.

## Git plan

### Tracking issue

Title: `Analyzer/generator: tiered redirect preferences`

Suggested body checklist:

- [ ] Define and validate `redirect_tiers` in `config.yaml` and
      `config.fixed.yaml`.
- [ ] Add a shared redirect rank/turn/tier classifier.
- [ ] Price same-hand turn trigrams by tier in the model/generator.
- [ ] Report per-tier and per-pattern redirect percentages in `analyze` and
      `worst`.
- [ ] Keep calibration/audit on the legacy fitted curve.
- [ ] Refresh regression fixtures and document the tier curve and fallback.
- [ ] Verify `just build`, `just test`, and `just check`.

Labels: `analyzer`, `generator`, `config`, `enhancement`.

### Branch

Create `analyzer/redirect-tiers` from `main` (matching the existing
`analyzer/*` branch convention).  If the work is split across people, use
short-lived stacked branches that all merge into `analyzer/redirect-tiers`
first:

- `analyzer/redirect-config` — schema/parser/classifier
- `analyzer/redirect-model` — model and generator pricing
- `analyzer/redirect-report` — analyze/worst output and docs
- `analyzer/redirect-baseline` — fixtures and CI cleanup

Do not merge the integration branch to `main` until the baseline refresh and
full `just check` pass.

### Commit sequence

1. `docs: plan tiered redirect preferences`
   - add this plan only.

2. `config: add redirect tier preferences`
   - `Config` field, parser, validation, shared rank/turn helper, parser tests;
   - add the section to `config.yaml` and `config.fixed.yaml`.

3. `model: price redirects by configured tier`
   - `Scales` table, `tri_many_into` lookup, fallback behavior, calibration
     isolation, model tests.

4. `analyzer: report redirect tiers and patterns`
   - structured classifier, per-tier/per-pattern keys, text output, `worst`
     label, analysis and command tests that do not depend on refreshed baselines.

5. `test: refresh redirect tier baselines`
   - updated parity/baseline/score fixtures with a short note in the PR body
     listing why scores moved.

6. `docs: document redirect tier preferences`
   - README configuration/analysis sections, config comments if not already in
     commit 2, and mark the old redirect/weak taxonomy as legacy.

7. Optional follow-up: `tools: mirror redirect tiers in Python fitting`
   - only if the dev tools are expected to match the Rust model.

### PR checklist

- [ ] The branch is based on current `main`.
- [ ] `git diff main...analyzer/redirect-tiers` contains no unrelated formatting.
- [ ] `just build`, `just test`, and `just check` pass.
- [ ] `check-baseline` passes with refreshed fixtures.
- [ ] The PR links the tracking issue and includes the tier cost table.
- [ ] The PR body calls out the 12 unlisted fallback patterns and the legacy
      output-key deprecation.

## Acceptance criteria

- The exact tier list from this request loads from `config.yaml` without
  warnings.
- For equal global scale, generator redirect costs are strictly ordered
  tier 1 < tier 2 < tier 3 < tier 4 < tier 5.
- `analyze` shows each configured redirect pattern and each tier separately,
  plus `total_redirects`.
- A same rank triple on the left and right hand is classified into the same
  tier.
- Removing `redirect_tiers` restores the current legacy scores.
- Calibration/audit results are unchanged when only `redirect_tiers` changes.
- All existing tests plus the new redirect tests pass; fixtures are refreshed
  only for intentional model changes.

## Risks and open questions

- **Unlisted patterns.** The request lists 16 of 28 possible turns.  The plan
  recommends legacy fallback; confirm whether the user instead wants unlisted
  redirects assigned to the weakest tier.
- **Cost curve.** Anchoring tier 1 to `W_REDIRECT` and the last tier to
  `W_REDIRECT_WEAK` is the lowest-risk default.  Confirm the user does not want
  a sharper curve (for example tier 5 much worse than the old weak redirect).
- **Chosen-fingering feedback.** The analyzer derives patterns from the chosen
  fingering in `tables.tri_f`.  Changing redirect prices can change which
  fingering wins, so tier percentages may not exactly equal a static count of
  physical key sequences.  This is existing behavior but should be documented.
- **Baseline churn.** Every enabled redirect cost changes the trigram table and
  therefore reference scores.  Keep the config feature opt-in and update
  fixtures deliberately rather than hiding the change.
- **Legacy output keys.** Keeping `redirect` / `redirect_weak` avoids an
  immediate breaking change but leaves two taxonomies in the report.  Mark
  them deprecated and remove them in a follow-up.
