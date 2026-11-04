# Preference Alignment Tasklist

Planning document for aligning `shirolyzer` with the user's stated comfort
preferences. No implementation is part of this document.

- Base: `main` at `3e09bd3`
- Existing aligned work: `8b41877` (rest keys, finger strength, chosen balance,
  score breakdown) and `3e09bd3` (new reference layouts)
- Primary corpus: `corpora/shai/parsed` (English, 418M letters)
- Secondary corpus: `corpora/douglas/parsed` (Spanish, 8.6M letters)
- Repository policy: `main` is the only long-lived branch

## Fixed decisions

- Keep the corpus configuration exactly as it is: SHaI primary, Douglas
  secondary.
- Both hands have equal strength. Any left/right asymmetry comes from ANSI
  geometry and reachability, not from a hand-strength multiplier.
- Geometry and finger strength are separate concepts:
  - Geometry covers travel, reach, row/column offsets, and key accessibility.
  - Finger strength is an explicit per-finger penalty on key presses.
- Finger balance is a means, not a primary goal. The primary goal is to put the
  most frequent letters and n-grams on the strongest fingers and the most
  comfortable motions.
- Use corpus measurements and reference-layout results to choose weights. Do
  not guess magnitudes when a sweep can show the effect on layout structure.
- Keep `shirolyzer/tests/fixtures/config.reference.yaml` frozen unless an issue
  explicitly says to refresh reference baselines.
- Do not modify `shirolyzer/config/references.toml` except when a new reference
  layout is intentionally accepted.

## Definitions

### Finger notation

`L`/`R` selects the hand. `P`, `R`, `M`, `I` select pinky, ring, middle, and
index. Examples: `LI` = left index, `RR` = right ring.

### `chosen` vs `default` balance source

The earlier ambiguity, resolved:

- `default`: for each key, use the fixed physical fingermap from the geometry
  file (`default_finger`) and accumulate corpus key frequency. Alternative
  fingerings are ignored for balance.
- `chosen`: use the fingers actually selected by the trigram DP for each corpus
  trigram. The DP selects only from allowed/blocked alternative fingerings and
  reachable fingers; `chosen` does **not** bypass the alternative-fingering
  table or go against it. It measures the fingers that were really used.

The current default is `chosen`. See work item P2 for the documentation and
regression tests that make this semantics explicit.

### Comfort metrics

The current model exposes SFB/SFT, scissors, redirects, rolls, alternation,
and balance as separate terms. The preference work adds explicit measurements
for:

- frequency-weighted finger strength (letters, bigrams, trigrams),
- scissor severity bands,
- redirect tier distribution,
- roll direction distribution and quality,
- top-letter and top-n-gram finger assignments.

## Guiding preferences

1. Worst scissors should be treated at roughly the same severity as SFB/SFT.
   Milder scissors should rank below SFBs, with severity decreasing smoothly.
2. Redirects should be as low as possible without harming more important
   statistics. The best redirects should be effectively free; `3-2-3` should
   be a hair worse than the best tier; later tiers must be progressively more
   penalized.
3. Frequent letters should be handled by the strongest fingers. Frequent
   bigrams/trigrams should use perfect rolls and comfortable motions with the
   strongest fingers. Balance only prevents extremes.
4. Index and middle fingers are strong and dexterous. Ring and pinky are weaker,
   and direction preferences grow stronger as the fingers involved get weaker.
5. Index↔middle rolls are direction-insensitive. Middle↔ring has a slight
   preference for ring→middle. Ring↔pinky has a strong preference for
   pinky→ring. Pinky→index is the generally natural direction.
6. H and Y must be usable by left index when the bigram/trigram context demands
   it. H is equally comfortable as right index or left index. Y left index is
   more comfortable than Y right middle.
7. M may use right index or right ring but never right middle; right ring is
   more comfortable because it travels less. X may use left ring or left index;
   left ring is preferable.
8. Keep the current corpus setup and do not add per-hand strength differences.

## Planned metrics and guardrails

Every candidate change should be evaluated on SHaI and Douglas with these
metrics:

- SFB/SFT rate and weighted cost,
- scissor count by severity band and weighted cost,
- redirect count by tier and weighted cost,
- roll count by pair and direction,
- top-25 letters and their finger assignments,
- top bigrams/trigrams and their finger sequences,
- frequency-weighted finger strength,
- maximum per-finger balance deviation,
- primary and secondary corpus scores on retained reference layouts,
- layout identity and distance from current reference layouts.

Guardrail: a change may improve its target metric only if it does not silently
regress a higher-priority metric. Any intentional trade must be documented with
before/after numbers and reference-layout diffs.

## Git workflow

For every work item:

1. Create a GitHub issue with the proposed title, labels, user preference
   references, and acceptance criteria.
2. Branch from an up-to-date `main`:
   `git switch main && git pull --ff-only && git switch -c <branch>`
3. Keep commits logical and imperative:
   `<area>: <lowercase description>`
   Areas: `model`, `config`, `analyze`, `compute`, `tests`, `docs`, `tools`.
4. Run `just check` before the final commit on the branch.
5. Run the ignored compute smoke test when the change touches search or
   scoring:
   `cargo test --manifest-path shirolyzer/Cargo.toml --release --test commands compute_smoke -- --ignored`
6. Check off the completed work item's boxes, then commit the changes in
   logical commits. Open a PR referencing the issue, or merge the branch
   directly when the work is already reviewed.
7. Merge the topic branch into `main` with a merge commit to preserve the
   existing history style. Delete the local and remote topic branch and push
   `main`.
8. Immediately start the next work item on a fresh branch from the updated
   `main`. Do not stop between work items with completed work left unmerged.
   End state after the final item: only `main` remains locally and on
   `origin`.

Planned issue/branch map:

| ID | Issue | Branch | Depends on | User refs |
|----|-------|--------|------------|-----------|
| P1 | Add preference metrics and calibration harness | `tools/preference-metrics` | `main` | 2, 3, 5, 9, 15 |
| P2 | Document and test balance-source semantics | `docs/balance-semantics` | P1 | 4 |
| P3 | Correct H/Y/M/X fingering permissions | `model/fingering-preferences` | P1 | 7, 13, 14 |
| P4 | Correct Y/U and U/H rulings | `tests/ruling-corrections` | P3 | 13, 14 |
| P5 | Make roll direction frequency- and strength-aware | `model/roll-direction` | P1, P3 | 8, 9, 15 |
| P6 | Tier scissors by severity | `model/scissor-tiers` | P1 | 1 |
| P7 | Add per-tier redirect weights and floor | `model/redirect-tiers` | P1 | 2, 16 |
| P8 | Replace balance-first goal with frequency-weighted finger priority | `model/finger-priority` | P1, P5, P6, P7 | 3, 5, 6 |
| P9 | Final preference calibration and regression lock | `release/preference-calibration` | P2-P8 | all |

## P1. Add preference metrics and calibration harness

**Why:** Several answers depend on measured tradeoffs, not intuition. The user
explicitly does not know how roll-weight magnitudes affect the final layout.

**Branch:** `tools/preference-metrics`

**Tasks:**

- [x] Extend `analyze` (or add a companion report) with the metrics listed in
      "Planned metrics and guardrails".
- [x] Report scissor costs by severity band, not only aggregate scissors.
- [x] Report redirect counts and costs per configured redirect tier.
- [x] Report roll counts and costs by finger pair and direction.
- [x] Report top-25 letters with their selected fingers.
- [x] Report top bigrams/trigrams with finger sequences and their term costs.
- [x] Add frequency-weighted finger-strength metric using a configurable
      strength vector.
- [x] Add a sweep tool that runs config variants across seeds and both corpora
      and emits machine-readable results.
- [x] Add math tests for the new metrics.

**Planned commits:**

1. `analyze: report preference metrics`
2. `tools: add preference calibration sweep`
3. `docs: document preference metrics`

**Acceptance:**

- One command produces all metrics for a layout and both corpora.
- Metrics do not change scoring or existing report baselines.
- Later issues can compare config variants without ad hoc scripts.

## P2. Document and test balance-source semantics

**Why:** The user asked for a precise explanation of what `chosen` means and
whether it conflicts with the fingermap or alt-fingering list.

**Branch:** `docs/balance-semantics`

**Tasks:**

- [x] Add a documentation section that defines `default` and `chosen` exactly as
      in "Definitions" above.
- [x] Add a worked example showing a context where `chosen` uses an allowed
      alternative finger and `default` uses the physical fingermap.
- [x] Add a regression test proving that `chosen` changes when an allowed alt
      fingering is used, while `default` does not.
- [x] Confirm that both modes still respect allow/block rules and reachability.
- [x] Keep `chosen` as the default unless the user explicitly changes it after
      reviewing the example.

**Planned commits:**

1. `docs: define chosen and default finger balance`
2. `tests: pin chosen balance to alternate fingerings`

**Acceptance:**

- Documentation answers the user's original question without ambiguity.
- The test demonstrates the difference and prevents future semantic drift.

## P3. Correct H/Y/M/X fingering permissions

**Why:** These are explicit corrections to the model's current fingering
behavior.

**Branch:** `model/fingering-preferences`

**User preferences:**

- H: right index or left index, equal comfort, context-dependent.
- Y: right middle or left index; left index is more comfortable than right
  middle.
- M: right index or right ring, never right middle; right ring is more
  comfortable because of less travel.
- X: left ring or left index; left ring preferable.

**Tasks:**

- [x] Audit current permissions and reachability for H/Y/M/X in
      `config.yaml`, `alt_fingering`, `conditions`, and geometry.
- [x] Allow Y left index where reachable; prefer it over Y right middle.
- [x] Reassess the existing `Y:LM` allowance. The stated allowed set is RM or
      LI. Remove or demote `Y:LM` only after checking existing rulings and
      reference layouts.
- [x] Allow H left index broadly in contexts where it is comfortable, not only
      in the current special-case entries.
- [x] Block M right middle everywhere; allow M right ring; prefer right ring
      over right index where the context allows.
- [x] Keep X left ring as the default and ensure X left index is allowed but
      slightly less preferred.
- [x] Add audit probes for each corrected key.
- [x] Verify that all other key mappings remain unchanged.

**Planned commits:**

1. `config: correct H Y M X fingering permissions`
2. `model: price preferred alternative fingerings`
3. `tests: pin H Y M X fingering preferences`

**Acceptance:**

- H uses RI or LI as context requires.
- Y prefers LI over RM and does not freely use LM.
- M never uses RM and prefers RR when reachable.
- X prefers LR with LI as a viable alternative.
- Unmentioned keys retain current behavior.

## P4. Correct Y/U and U/H rulings

**Why:** The user confirmed the old rulings are wrong or incomplete for these
contexts.

**Branch:** `tests/ruling-corrections`

**User preferences:**

- Y→U: prefer `LI-RM` over `LM-RM`; right-middle→right-ring is very
  uncomfortable and should be impossible.
- U→H: left-index H is acceptable; the old "impossible" ruling is wrong.
- Both H and Y should be altable by left index when bigram/trigram context
  demands it.

**Tasks:**

- [x] Update the Y→U and U→Y expected fingerings from the current `LMRM` /
      `RMLM` behavior to the user's preferred `LIRM` / `RMLI` forms.
- [x] Keep right-middle→right-ring Y→U impossible.
- [x] Replace the U→H "must be impossible" ruling with acceptance of the
      left-index H form (`RMLI`) while still rejecting the uncomfortable
      right-index form (`RMRI`).
- [x] Update the corresponding audit checks (`r12_YU_*`, `r12_UY_*`, `r9_UH`,
      `r9_HU`).
- [x] Add regression tests for both directions.

**Planned commits:**

1. `config: allow left-index Y and H in Y U U H contexts`
2. `tests: correct Y U and U H rulings`
3. `tests: add Y U and U H regression probes`

**Acceptance:**

- `audit` reports the corrected expectations.
- Y→U selects `LI-RM` rather than `LM-RM`.
- U→H accepts `RMLI` and rejects `RMRI`.
- No unrelated rulings change.

## P5. Make roll direction strength-aware

**Why:** Direction preference currently does not match the user, especially for
W/D and A/S, and the preference magnitude should depend on finger weakness.

**Branch:** `model/roll-direction`

**User preferences:**

- Index↔middle: direction does not matter.
- Middle↔ring: very slight preference for ring→middle over middle→ring; this is
  only a tie-breaker.
- Ring↔pinky: strong preference for pinky→ring over ring→pinky.
- Direction matters increasingly as the fingers involved get weaker.
- W→D is preferred over D→W. If W is alted to LI with D as LM, the pair becomes
  index→middle and direction should become a tie.
- A→S (pinky→ring) is preferred over S→A (ring→pinky). The current values make
  S→A much cheaper, which is backwards.

**Tasks:**

- [x] Audit the semantics of `r_MI`, `r_RM_in`, `r_RM_out`, `r_PR_in`,
      `r_PR_out`, and `m_*` before changing values.
- [x] Define a direction-bias function that scales with finger weakness.
- [x] Force index↔middle direction costs to tie.
- [x] Make ring→middle slightly preferred over middle→ring.
- [x] Make pinky→ring strongly preferred over ring→pinky.
- [x] Invert W/D so W→D is slightly preferred over D→W.
- [x] Ensure the W:LI + D:LM alternative makes W/D direction-insensitive.
- [x] Invert A/S so A→S is preferred over S→A.
- [x] Measure the final A/S ratio with P1; do not assume the current nearly
      double gap is correct.
- [x] Reconcile direction bias with `roll_tiers` so unlisted pairs retain the
      intended model weights.

**Planned commits:**

1. `model: scale roll direction by finger strength`
2. `config: prefer W D and pinky-ring directions`
3. `tests: pin roll direction preferences`

**Acceptance:**

- IM and MI tie.
- RM is slightly cheaper than MR.
- PR is clearly cheaper than RP, with the gap justified by a sweep.
- W→D is cheaper than D→W under the default fingering.
- A→S is cheaper than S→A.
- Alt-fingered W/D behaves as an index↔middle tie.

## P6. Tier scissors by severity

**Why:** The user wants worst scissors treated like SFB/SFT, with milder
scissors ranking below SFBs. The current model has continuous scissor costs but
no explicit severity tiers comparable to `redirect_tiers`.

**Branch:** `model/scissor-tiers`

**Tasks:**

- [ ] Audit the current scissor cost sources: lateral spread, direction,
      full-span conditions, reverse-scissor restrictions, and hard blocks.
- [ ] Define named severity bands from most to least comfortable.
- [ ] Add a `scissor_tiers`-style configuration or severity-band multipliers.
- [ ] Weight the worst scissor band at approximately the SFB/SFT priority.
- [ ] Make each milder band progressively cheaper than SFBs.
- [ ] Preserve hard impossibilities for full-span M/R and other blocked forms.
- [ ] Report scissor counts and costs per band in `analyze`.
- [ ] Add ordering tests for representative scissor pairs.
- [ ] Run reference-layout comparisons before accepting the new weights.

**Planned commits:**

1. `model: expose scissor severity bands`
2. `config: tier scissor weights`
3. `analyze: report scissor severity distribution`
4. `tests: pin scissor severity order`

**Acceptance:**

- Worst scissor cost is comparable to SFB/SFT.
- Mild scissor cost is below SFB/SFT.
- Scissor ranking matches user severity expectations.
- Hard-blocked scissors remain impossible.
- Reference layouts do not move unexpectedly without documented justification.

## P7. Add per-tier redirect weights and a floor

**Why:** The user wants best redirects effectively free, `3-2-3` only a hair
worse, later tiers progressively worse, and the total redirect rate as low as
possible without harming more important metrics.

**Branch:** `model/redirect-tiers`

**Tasks:**

- [ ] Replace the single `redirect_scale` with per-tier weights while keeping
      backward-compatible parsing.
- [ ] Set the best tier (`1-2-1`, `2-1-2`, `1-3-1`, `3-1-3`) at or near zero.
- [ ] Set `3-2-3` to a very small tie-breaker penalty.
- [ ] Make the remaining tiers monotonically more expensive, worst last.
- [ ] Sweep weights with P1 to minimize redirect rate subject to guardrails on
      SFB, scissors, rolls, and finger priority.
- [ ] Report the redirect tier distribution in `analyze`.
- [ ] Document the chosen floor and the measured tradeoff.

**Planned commits:**

1. `config: add per-tier redirect weights`
2. `model: apply redirect weights per tier`
3. `tools: sweep redirect tier weights`
4. `docs: document redirect floor and tradeoffs`

**Acceptance:**

- Best redirects are effectively free.
- `3-2-3` is only marginally penalized.
- Remaining redirects are ordered by severity.
- Redirect rate is at the measured minimum without guardrail regressions.

## P8. Replace balance-first goal with frequency-weighted finger priority

**Why:** The user's primary goal is not equal finger usage. It is to put the
most frequent letters and n-grams on the strongest fingers and the most
comfortable motions, with balance only as a guardrail.

**Branch:** `model/finger-priority`

**Tasks:**

- [ ] Add a frequency-weighted finger-strength objective using top-letter,
      top-bigram, and top-trigram frequency.
- [ ] Keep geometry separate from strength; do not encode strength into travel
      costs.
- [ ] Use the same strength vector for both hands.
- [ ] Calibrate strength values with P1. Expected anatomical ordering:
      index/middle strongest, ring weaker, pinky weakest.
- [ ] Demote `finger_balance` to a lower priority tier or tie-breaker weight.
- [ ] Add a balance guardrail so the optimizer cannot ignore a finger entirely.
- [ ] Report top-25 letters and their finger assignments in `analyze`.
- [ ] Report top bigrams/trigrams and their roll/redirect/scissor quality.
- [ ] Verify that frequent n-grams prefer perfect rolls and avoid redirects.
- [ ] Add tests for the frequency-weighted objective and balance guardrail.

**Planned commits:**

1. `analyze: weight finger load by corpus frequency`
2. `model: reward strong fingers on frequent keys`
3. `config: demote finger balance to a tie-breaker`
4. `tests: pin frequency-weighted finger behavior`

**Acceptance:**

- Frequent letters concentrate on index and middle fingers.
- Common n-grams prefer perfect rolls using strong fingers.
- Balance remains bounded but is no longer a primary objective.
- Geometry and strength remain independent inputs.
- Both hands use the same strength vector.

## P9. Final preference calibration and regression lock

**Why:** After the structural changes, the weights must be calibrated together
on both corpora and locked with regression coverage.

**Branch:** `release/preference-calibration`

**Tasks:**

- [ ] Run the P1 sweep over the final config dimensions.
- [ ] Compare SHaI and Douglas metrics and reference-layout scores.
- [ ] Select weights at the measured tradeoff boundary, not by intuition.
- [ ] Update only the default `config.yaml` values that the sweep supports.
- [ ] Refresh `tests/fixtures/config.reference.yaml` only if an intentional
      baseline change is approved.
- [ ] Add regression tests for the final preference ordering.
- [ ] Update README and config comments with the final semantics.
- [ ] Run `just check`.
- [ ] Run the ignored `compute_smoke` test.
- [ ] Run `analyze` and `audit` on a retained layout and record final metrics.
- [ ] Merge to `main`, delete the topic branch, and push.

**Planned commits:**

1. `config: apply preference calibration`
2. `docs: record preference calibration results`
3. `tests: lock final preference regressions`

**Acceptance:**

- Final metrics match the documented tradeoffs.
- All tests, clippy, and formatting pass.
- `main` is the only remaining branch.
- The working tree is clean after the merge.

## Response-to-work-item map

| Response | Summary | Work item |
|----------|---------|-----------|
| 1 | Scissor severity: worst ≈ SFB, milder below SFB; alternation weakest | P6, P9 |
| 2 | Redirect rate as low as possible without harming higher stats | P7, P9 |
| 3 | Frequent letters/n-grams on strong fingers; balance is a means | P8 |
| 4 | Need exact `chosen` vs `default` meaning | P2 |
| 5 | Geometry and strength are separate; use strongest fingers for frequent input | P8 |
| 6 | ANSI reach asymmetry; equal hand strength; balance is secondary | P8 |
| 7 | H RI/LI; Y RM/LI with LI preferred; M RI/RR never RM; X LR/LI with LR preferred | P3 |
| 8 | Prefer W→D; index↔middle direction tie; alt W/D removes direction | P5 |
| 9 | Prefer pinky→ring, but magnitude needs measurement | P5, P1 |
| 10 | O→K preference aligned | P9 regression only |
| 11 | X+A permissions aligned | P9 regression only |
| 12 | Full-span M/R scissor impossible; aligned | P9 regression only |
| 13 | Y→U prefer LI-RM; RM→RR impossible | P3, P4 |
| 14 | U→H left-index acceptable; old ruling wrong | P3, P4 |
| 15 | RM > MR slightly; IM/MI tie; PR > RP strongly; direction scales with weakness | P5 |
| 16 | Redirect ranking aligned; best free, `3-2-3` near-free, rest increasing | P7 |
| 17 | Keep SHaI primary and Douglas secondary unchanged | P9 guardrail only |
