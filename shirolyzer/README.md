<div align="center">

# shirolyzer【白ライザー】

###### Layout analyzer & generator based on preferential, context-dependent fingering. Initially built for the Shiro layout.

<!-- (image) -->

</div>

---

### Quick anchors

- [Introduction](#introduction)
    - [About 白ライザー](#about-白ライザー)
- [Generation](#generation)
    - [Corpora](#corpora)
    - [Primary sacrifice](#primary-sacrifice)
    - [Alt-fingers & Context](#alt-fingers--context)
    - [Compute scheduling](#compute-scheduling)
    - [Polishing](#polishing)
    - [Terminal output](#terminal-output)
    - [Measuring variance](#measuring-variance)
    - [Model parameters](#model-parameters)
    - [Priority tiers](#priority-tiers)
    - [Redirect tiers](#redirect-tiers)
    - [Roll tiers](#roll-tiers)
    - [Conditions](#conditions)
- [Analysis](#analysis)
    - [Preference metrics](#preference-metrics)
    - [Corpus provenance](#corpus-provenance)
- [Development](#development)
    - [Tests and the reference model](#tests-and-the-reference-model)

---

# Introduction

## About 白ライザー

The **shirolyzer** is an alt keyboard layout analyzer and generator that is highly customizable to preferentials.

Mainly, it features user-defined condition/preference/restriction configuration in `shirolyzer/config/config.yaml`:

**Physical layout and fingering**:

- Physical layout geometry is defined in `shirolyzer/geometry/*.yaml`; the config selects one:
  ```yaml
  general:
    geometry: geometry/ansi.yaml
  ```
- Custom fingermap definition, using hand/finger tokens `LP/LR/LM/LI` and `RI/RM/RR/RP`.
- Finger rest keys definition.
- Alt-finger ngram rules that override the fingermap for specific physical-key sequences, including explicit cross-hand fingerings.
- Configurable fingering mode: context-dependent `flexible` or physical-fingermap `fixed` (`general.fingering`).
- Conditional preferences/restrictions for scissors, spread, separation, and crossing.

**Tuning**:

- Primary and secondary corpora, with weight-specific blending.
- Primary sacrifice limits, expressed as score and/or per-metric percentages.
- Optional per-hand character restrictions (`hand_restrictions`), so selected characters are only placed on one hand during generation.
- Priority tiers for selected objective terms and ranked redirect preference tiers.

## Generation
> shirolyzer's layout generation capabilities and specifications

### Corpora

On layout generation, it is capable of taking corpora under different weightings:

- **Primary:** One or more primary corpora are evaluated together ("a priori") in a weighted blend (e.g. 100, 50:50, 50:25:25, 80:20:20).

- **Secondary:** Any number of corpora can optionally be evaluated _after_ the initial generation ("a posteriori"). The generator then optimizes for the secondary corpora without exceeding the allowed primary sacrifice.

### Primary sacrifice

`general.primary_sacrifice` accepts either a number/percentage (score only) or a mapping:

```yaml
general:
  primary_sacrifice:
    score: 0.00023%
    sfbs: 1%
    scissors: 1%
    redirects: 1%
    rolls: 1%
    alternation: 1%
    finger_balance: 1%
```

Percentages are relative to the best primary layout. Lower-is-better metrics (`sfbs`, `scissors`, `redirects`, `finger_balance`) may increase by the given percentage; higher-is-better metrics (`rolls`, `alternation`) may decrease by it. `sfb`, `sfts`, and `sfbs` are aliases for the combined SFB/SFT budget. That budget counts ordinary same-finger bigrams and adjacent same-finger trigrams (Sfb and BadSfb lumped together) at 1x, and true all-three-same-finger trigrams (Sft) at 2x. `score` is optional only when every metric is specified.

### Alt-fingers & Context

The generator takes user-defined alt-fingers into account as optimization criteria. These override the selected fingermap.

Alts are defined from physical key positions rather than characters, so the generator can apply them to the evolving letter arrangement:

```yaml
alt_fingers:
  - {ngram: [F, J], fingers: [LP, RI], mode: allow}
  - {ngram: [S, T, R], fingers: [LR, LM, LI], mode: block}
```

- Matching is exact and ordered. A longer `allow` rule overrides shorter rules only where their matched ranges overlap; non-overlapping shorter rules remain active, and overlapping `block` rules all stay active.
- `allow` restricts an ngram to the listed fingering; `permit` (also spelled `license`) adds that fingering as an option without excluding others; `block` forbids that exact fingering while leaving other fingerings available.
- Listed fingers for `allow` are mandatory. An explicit rule never silently falls back to a different default fingermap.
- An explicit `allow` or `permit` rule also licenses a finger that geometry would otherwise mark unreachable; the normal travel weights still price that reach.
- When the same ngram is defined more than once, earlier definitions are preferred; later definitions act as fallbacks.
- Context-key defaults are listed first, followed by other fingers on the same hand as fallbacks.

#### Cross-hand fingerings

The default fingermap is same-hand: every key uses a finger on its own hand. Cross-hand fingering is opt-in through an explicit `alt_fingers` token, for example:

```yaml
alt_fingers:
  - {ngram: [H], fingers: [LI], mode: permit}
  - {ngram: [Y], fingers: [LI], mode: allow}
  - {ngram: [Y], fingers: [RM], mode: allow}
```

Internally the tokens map to eight finger states ordered by `hand * 4 + finger`: `LP=0`, `LR=1`, `LM=2`, `LI=3`, `RP=4`, `RR=5`, `RM=6`, `RI=7`. A bare `P/R/M/I` token still means the physical key's own hand, so the shipped fingermap and existing rules keep their same-hand meaning. Cross-hand travel is priced from absolute key/finger positions, not from a hand-relative mirror, and cross-hand states remain unavailable unless a rule explicitly allows them. An explicit `allow` or `permit` rule may name a finger outside the geometry reach map; the scorer then prices the reach with the normal travel weights instead of rejecting it. Same-hand pair terms use the fingering hand, so two cross-body keys typed by fingers of the same hand are still treated as a same-hand pair.

#### Fingering modes

Finger selection is user-defined through `general.fingering`:

```yaml
general:
  fingering: flexible   # default: any reachable finger, subject to alt_fingers
  # fingering: fixed    # always use the fingermap; ignore alt_fingers
```

- `flexible` lets the scorer choose a finger per bigram/trigram, using `alt_fingers` as constraints. This is shirolyzer's context-dependent analysis.
- `fixed` forces every key to its `fingermap` finger. Alt rules are ignored, so the resulting metrics are directly comparable to fixed-fingering analyzers such as oxeylyzer. Conditions and the fitted model still apply.

Two configs are shipped:

- `shirolyzer/config/config.yaml` — flexible (the default).
- `shirolyzer/config/config.fixed.yaml` — fixed physical/default fingering with no alt rules.

Switch between them with `--config`:

```sh
shirolyzer analyze "layout" --config shirolyzer/config/config.fixed.yaml
shirolyzer compare --config shirolyzer/config/config.fixed.yaml
```

### Compute scheduling

`compute` runs random anneals, keeps the best distinct layouts, refines them
with iterated local search (ILS), and then polishes the near-equal winners.
Useful flags:

- `--runs` (default `28`), `--iters` (default `200000`), `--ils` (default `8`)
  — restart count, anneal length, and ILS rounds.
- `--ils-candidates` (default `4`) — how many top layouts advance to ILS.
  Candidates are chosen by letter placement diversity (at least four displaced
  letter keys), so layouts that differ only in punctuation are not treated as
  independent candidates.
- `--batch-size` (default `8`) — random starts per batch. Batches run in
  parallel internally and sequentially with each other, which lets the
  stopping rules below run between batches.
- `--stop-hits` (default `0`, disabled) — stop once the best letter placement
  has been independently reached by this many random starts.
- `--stop-stale-batches` (default `0`, disabled) — stop once the best primary
  score has not strictly improved for this many consecutive batches.
- `--time-budget` (for example `90s`, `5m`, `1h`) — stop starting new work after
  this long and report the best result so far. Runs under a time budget execute
  sequentially instead of in parallel.
- `--valid-tolerance` (default `0.002`), `--secondary-tolerance` (default
  `0.005`), `--min-layout-distance` (default `4`), `--max-valid` (default `8`)
  — how near-equal, diverse layouts are collected into the valid-layout report.
- `--secondary-rounds` (default: same as `--ils`) — perturbation rounds per
  candidate in the secondary phase. `0` only scores the candidates.
- `--seed` — reproduce a run. Without it a seed is generated and printed in the
  header.
- `--verbose` / `-v` — list every distinct layout from the anneal stage with its
  layout string instead of only the top eight.

#### Starting layouts

Searches start from random layouts only. Known layouts are never used as warm
starts unless you ask for them:

- `--seed-layout "<30 characters>"` — start from a layout you provide (repeatable).
- `--seed-reference NAME` — start from a named entry in the references file
  (repeatable; an unknown name is an error).
- `--seed-all-references` — start from every reference that satisfies the
  configured hand restrictions.

Seed layouts that violate `hand_restrictions` are skipped with a warning. The
references file is still used to name and de-duplicate results.

The `hits` column counts independent starts that reached the same letter
placement, ignoring punctuation. Punctuation is placed canonically in the
final layout after the letter search, because punctuation positions do not
affect the primary score.

### Polishing

Polishing is on by default. Each round perturbs the best layout found so far,
then quenches, anneals, and quenches again. It repeats until a round fails to
strictly improve the layout, so the number of rounds adapts to the layout
instead of being fixed:

- `--polish-rounds` (default `unlimited`) — maximum rounds per candidate. Pass a
  number to cap it, or `0` to disable polishing. Polishing always stops early
  once a round finds nothing better, so `rounds=1` in the output means the first
  round found no improvement.
- `--polish-candidates` (default `4`) — how many layouts are polished. They are
  selected with the same tolerance and diversity rules as the valid-layout
  report.

### Terminal output

`compute` prints one section per stage:

- **Anneal** — the best distinct layouts sorted by primary score, with the number
  of starts that reached each (`hits`). `final` is the score the search tracked
  while annealing and `primary` is the score recomputed from scratch for the
  same layout; the two are expected to match.
- **ILS** — each candidate's score before and after, and the gain.
- **Polish** — before/after scores and the rounds used per candidate.
- **Secondary** — candidates scored against the secondary corpora within the
  primary sacrifice budget (only when secondary corpora are configured). Each
  distinct letter placement is optimized once: an ILS result and its unchanged
  polished copy, or layouts that differ only in punctuation, are not repeated.
- **Best layout** — the selected layout drawn with the same staggered geometry as
  `render`, plus any other valid layouts alongside it.

The footer lists the result files (`results/compute_*.{txt,json}`), the
`results/latest` layout pointer, any layouts appended to the references file
(named from `--name`, default `shiro`, plus the seed), and the elapsed time.
Use `--append-references PATH` to write new references somewhere else.

Progress bars are drawn on stderr, and only when it is a terminal. Color is used
only when stdout is a terminal; set `NO_COLOR=1` to turn it off. Piped or
redirected output is plain text, and the files written to `results/` are always
plain.

### Measuring variance

`just variance` builds the release binary and runs `compute` once per seed with
`shirolyzer/utils/compute_variance.py`, then reports the spread of
`best_primary`, its standard deviation, and the fraction of seeds within
`--tolerance` of the best result:

```sh
just variance --seeds 20
just variance --seeds 20 --runs 200 --iters 25000 --verbose
```

The script uses only the Python standard library. The recipe runs
`.venv/bin/python` by default; set `PYTHON=python3` to use another interpreter.
It is development tooling only and does not change the model or any baseline. Pass `--start-seed`, `--runs`, `--iters`, `--ils`,
`--ils-candidates`, `--batch-size`, `--stop-hits`, and
`--stop-stale-batches` to mirror the `compute` flags.

Measured on seeds `0..19` after the incremental-score fix (and before the
priority tiers were weighted), all runs reaching the
same letter placement. Re-run `just variance` after changing the objective:

| Budget | Spread | Std. dev. | Success rate |
| --- | --- | --- | --- |
| `--runs 28 --iters 200000` (default) | 0.000000000 | 0.000000000 | 100% |
| `--runs 200 --iters 25000` | 0.000000000 | 0.000000000 | 100% |
| `--runs 28 --iters 25000` | 0.005378908 | 0.001654132 | 85% |

With the shipped config (weighted priority tiers, `roll_tiers`, `redirect_scale: 3.0`),
seeds `0..3` at the default budget all reach the same layout; re-run
`just variance --seeds 20` to confirm on more seeds.

The default budget already reproduces the same optimum across every seed, and
the breadth-heavy budget matches it at roughly the same total iteration count.
Reducing the anneal depth alone is not enough, so the defaults stay at
`--runs 28 --iters 200000`, and the stopping rules stay disabled by default.

### Model parameters

The scoring model's coefficients live in `shirolyzer/config/model/`:

- `calibrated.json` — the fitted and audited reference model. The score baseline,
  `check-baseline`, `audit` and the tests are tied to this file, so don't edit it
  by hand; refit it with the tools in `shirolyzer/fitting/`.
- `fit_seed_A.json` and `anchor_pairs.json` — starting point and anchors for
  refitting.

Point `model.parameters_file` in the config at the file to use. To make your own
variant, copy `calibrated.json` and change the numbers you care about. Roll
preferences don't need a parameter file: use [`roll_tiers`](#roll-tiers).

### Priority tiers

`priority_list` groups objective terms into ordered tiers, most important first.
With N tiers, tier t (0 = highest) is multiplied by `1 + 0.5 * (N - t - 1)`, so
earlier tiers weigh more. Terms: `sfbs-sfts`, `scissors`, `redirects`, `rolls`,
`alternation`, `finger_balance`.

```yaml
priority_list:
  - sfbs-sfts                # x3.0
  - scissors                 # x2.5
  - redirects                # x2.0
  - [rolls, finger_balance]  # x1.5  (tiers can hold several terms)
  - alternation              # x1.0
```

- The list is always applied as written. To weight every term equally, put them
  all in a single tier.
- A term left out of the list is switched off (multiplier 0), and `compute`
  prints a warning when that happens.
- The effective multipliers are printed at the top of every `compute` run.
- Tiers scale a whole category. How bad an individual pattern is within it (for
  example harmless versus severe scissors, or which redirects are worst) comes
  from the model's cost curves, the `conditions` list and `redirect_tiers`.

### Redirect tiers

`redirect_tiers` ranks same-hand redirect patterns from most preferred to most avoided. Fingers are numbered per hand (`1=index`, `2=middle`, `3=ring`, `4=pinky`), and every entry must be a direction reversal such as `1-2-1` or `2-1-3`.

```yaml
redirect_tiers:
  - ["1-2-1", "2-1-2", "1-3-1", "3-1-3"]
  - ["3-2-3"]
  - ["2-1-3", "3-1-2", "2-3-2", "1-4-1"]
  - ["4-1-4", "4-2-4", "2-4-2"]
  - ["3-4-3", "4-3-4", "3-2-4", "4-2-3"]
```

Tier costs are spaced automatically between the calibrated `W_REDIRECT` and `W_REDIRECT_WEAK` endpoints, so tier 1 is the cheapest and the last tier is the most expensive. Same-hand turns that are not listed keep the legacy cost (pinky involved means the weak value, otherwise the regular value). Calibration and audit always use the legacy fitted curve, independent of this preference layer.

`redirect_scale` (optional, default `1.0`) multiplies every redirect cost on top of the `redirects` priority tier; `compute` prints the effective weight at the top of a run. Redirect costs are small next to the roll and alternation rewards, so with the tier weights alone the best layouts carry roughly 5.5% of trigrams as redirects. `redirect_scale: 3.0` brings that to roughly 2.5%. Raise it to avoid redirects harder, lower it to accept more. It must be greater than 0.

### Roll tiers

`roll_tiers` ranks directed same-hand finger pairs from the most to the least rewarded roll. A pair is `"from-to"` in the same per-hand numbering as the redirect tiers (`1=index`, `2=middle`, `3=ring`, `4=pinky`), so `"1-2"` is index then middle and `"2-1"` is middle then index. Only neighbouring fingers can roll.

```yaml
roll_tiers:
  - ["1-2"]            # index -> middle: the most rewarded roll
  - ["2-1", "3-2"]     # middle -> index and ring -> middle
```

- With N tiers, tier t (0 = highest) earns a roll credit of `-0.60 * (N - t) / N` per pair, so a single tier earns the full `-0.60`. The credit still fades with key distance and is multiplied by the `rolls` priority.
- A listed pair replaces the model's own weight for that pair and direction (`r_MI`, `r_RM_in`, `r_RM_out`, `r_PR_in`, `r_PR_out`). Unlisted pairs keep the model's weight, and the calibrated model is unchanged when `roll_tiers` is absent.
- The unlisted direction bias scales with the weaker finger: index<->middle is a tie, ring->middle is slightly preferred over middle->ring, and pinky->ring is strongly preferred over ring->pinky.
- This is a per-pair preference, not a redirect penalty: a redirect such as `1-2-1` still collects the credit of each pair it contains. To avoid redirects, use `redirect_tiers` and `redirect_scale`.
- `analyze` reports the share of letter bigrams in each tier and each pair (`roll_tier_N` and `roll_A_B` in the JSON).

### Finger load and strength

`finger_balance` sets the target share of strokes for each finger. The shipped
target is P 9%, R 12.5%, M 14%, I 14.5% on both hands:

```yaml
finger_balance:
  LP: 9
  LR: 12.5
  LM: 14
  LI: 14.5
  RP: 9
  RR: 12.5
  RM: 14
  RI: 14.5
```

`finger_balance_source` selects which fingering supplies the load used by the
balance penalty. The penalty is off whenever `finger_balance` is absent from
`priority_list`, regardless of this setting.

- `chosen` (default): run the same context-dependent trigram DP used by
  `analyze`, then accumulate the fingers it actually selected. It respects
  `alt_fingers` allow/block rules, geometry reachability, and the selected
  fingering mode. It does not bypass those rules and does not invent a
  fingering just to improve balance.
- `default`: accumulate each letter's corpus frequency on the physical key's
  fixed fingermap finger. Alternative and context-dependent fingerings are
  ignored.

For example, place `H` on a key whose physical fingermap finger is `RR`, and
allow the left-index alternative:

```yaml
alt_fingers:
  - {ngram: [H], fingers: [LI], mode: allow}
```

With `finger_balance_source: default`, every `H` press contributes to `RR`.
With `chosen`, the DP follows the allowed rule and those same `H` presses
contribute to `LI`. If the rule is removed, blocked, or the alternative is
unreachable, `chosen` falls back to the permitted fingering (the physical
fingermap when no alternative is available); `default` is unchanged either
way. The regression tests in `src/search.rs` pin this difference and verify
that both modes respect the same allow/block and reachability rules.

`finger_strength` (optional) adds a constant cost to every press by selected
fingers. Bare `P`/`R`/`M`/`I` apply to both hands; explicit `LP`/`LR`/... tokens
override one hand. Values are non-negative and default to 0:

```yaml
finger_strength:
  LP: 0.30
  LR: 0.10
  RP: 0.45
  RR: 0.15
```

Higher values steer generation away from weak or uncomfortable fingers. The
term enters the key-effort table, so it also appears in the analyzer's
`key_effort` decomposition.

### Conditions

Condition rules are evaluated from top to bottom; the first matching rule wins. `allow: false` forbids a matching pattern and `allow: true` permits it. Unless `reverse: true` is specified, a scissor rule matches both regular and reverse scissors.

`fingers` matches when either endpoint is in the listed set. Use `pairs` when the rule must match both endpoints exactly, for example `pairs: [[LR, LP], [RR, RP]]` for ring/pinky pairs in either direction. Pair entries must use explicit hand/finger tokens.

The shipped condition list is a built-in default and stays disabled because the fitted model already implements those native rules. Editing the list enables the conditions engine for the edited list; the built-in defaults are kept in `src/config.rs` (and `utils/config.py`) so a config that still matches them stays disabled.

## Analysis
> Details about shirolyzer's layout analysis

The **shirolyzer** analyzer uses the same configuration, geometry, model, and fingering rules as the generator. It reports:

- bigram categories such as SFBs, scissors, and stretches;
- trigram patterns including rolls, redirects, alternation, true same-finger trigrams (Sft), and adjacent same-finger trigrams (Sfb/BadSfb lumped); repeated-character trigrams are also reported as an overlapping `repeat` diagnostic;
- when `redirect_tiers` is configured, per-tier and per-pattern redirect percentages plus `redirect_unclassified` for unlisted turns; the older `redirect`/`redirect_weak` categories remain as deprecated overlapping diagnostics;
- when `roll_tiers` is configured, the share of letter bigrams in each roll tier and pair; the six directed pair shares (`roll_1_2` ... `roll_4_3`) are always in the JSON;
- per-finger usage against the configured finger-balance targets;
- the pattern-model score, including priority-tier weighting;
- a per-term score decomposition (`score_breakdown` in JSON and a text table)
  that reconstructs the primary score from key effort, pair terms, trigram
  terms, gate penalties, and space costs.

`analyze` uses the most recent `compute` result when `--layout` is omitted. The same metrics drive primary-sacrifice enforcement during secondary optimization, so scores shown by the analyzer reflect the constraints the generator applied.

### Preference metrics

`analyze` also writes a `preference` block for the primary corpus and, when one
is configured, the secondary corpus. The text report prints a compact summary;
`--out-json` contains the full top-25 lists and all per-pattern values. These
metrics are observational: they do not feed back into the score or change any
baseline.

- `scissor_bands` groups scissors by their configured pair cost: `mild` below
  `3`, `moderate` from `3` to below `5`, `severe` from `5` to below `8`, and
  `extreme` at `8` or above. Each band reports count, share of corpus bigrams,
  and frequency-weighted cost. `scissor_categories` keeps the existing category
  breakdown. Adjust the thresholds in `SCISSOR_BAND_THRESHOLDS` when P6
  introduces configured severity tiers.
- `redirect_tiers` reports count, share, and weighted cost for each configured
  tier, with a `patterns` map for the individual turn patterns.
  `redirect_unclassified` covers turns outside every configured tier.
- `rolls` reports count, share, and weighted cost for the six directed adjacent
  finger pairs, including pairs whose cost is zero.
- `top_letters` lists the 25 most frequent letters with their key, configured
  default finger, selected-finger distribution, and weighted strength.
- `top_bigrams` and `top_trigrams` list the 25 most frequent n-grams with the
  selected finger sequence, term cost, and category.
- `finger_strength` reports frequency-weighted mean finger-strength penalty for
  letters, bigrams, and trigrams, plus the per-finger weighted distribution.
  The vector comes from the top-level `finger_strength` mapping (zero when it
  is omitted), so configs can be compared without editing code.

Use the calibration sweep to compare config variants across seeds and both
corpora:

```sh
just sweep --seeds 5 --out-json shirolyzer/results/preference_sweep.json
just sweep --variants sweep_variants.yaml --seeds 10
```

`shirolyzer/utils/preference_sweep.py` accepts a JSON or YAML list (or an object
with a `variants` list). Each variant has a `name` and optional `overrides`
using dotted YAML paths:

```yaml
- name: baseline
  overrides: {}
- name: redirect-5
  overrides:
    redirect_scale: 5.0
```

The script runs `compute` for each variant/seed, analyzes the selected layout
with both corpora, and writes the compute rows plus the full preference blocks
to one JSON file. Search budget flags mirror `compute` (`--runs`, `--iters`,
`--ils`, `--ils-candidates`, `--batch-size`, `--secondary-rounds`,
`--polish-rounds`, `--time-budget`).

### Corpus provenance

`analyze` records a `corpus_provenance` block for the primary and secondary corpora. Each entry names the resolved path, corpus kind, configured weight, per-file SHA-1 and combined hash, parser rule id, normalization rule, and n-gram totals. The text report prints a compact summary; `--out-json` contains the full record. Provenance is observational and never changes a score.

`compare` can read an external comparison bundle with `--corpus-json FILE`. The file is a single JSON object with `letters`, `bigrams`, and `trigrams` maps from n-gram strings to counts. This is a comparison-only input: it does not replace the native parser, and the config corpora still drive generation and the shipped baseline.

## Development

### Tests and the reference model

`just check` runs the tests, `cargo fmt --check` and `clippy`. The regression
baselines (`config/score_baseline.json`, the expected scores in `tests/`) measure
a fixed reference model, not your live `config.yaml`. The tests build their
config from `tests/fixtures/config.reference.yaml`, a frozen copy that uses
`model/calibrated.json`, with cross-hand rules and hand restrictions stripped at
test time.

Editing `config.yaml` or the priority tiers therefore never breaks
the suite. Only change the fixture, and regenerate the baselines
(`shirolyzer check-baseline --write`), when you deliberately change what the
reference model measures.

---

<div align="center">

</> with <3 by [akai_hana](https://github.com/akai-hana)

</div>
