<div align="center">

# shirolyzer【白ライザー】

###### Layout analyzer & generator based on preferential, context-dependent fingering. Initially built for the Shiro layout.

<!-- (image) -->

</div>

---

### Quick anchors

- [a](#a)
    - [b](#b)

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
- `allow` restricts an ngram to the listed fingering; `block` forbids that exact fingering while leaving other fingerings available.
- Listed fingers are mandatory. An explicit rule never silently falls back to the default fingermap.
- An explicit `allow` rule also licenses a finger that geometry would otherwise mark unreachable; the normal travel weights still price that reach.
- When the same ngram is defined more than once, earlier definitions are preferred; later definitions act as fallbacks.
- Context-key defaults are listed first, followed by other fingers on the same hand as fallbacks.

#### Cross-hand fingerings

The default fingermap is same-hand: every key uses a finger on its own hand. Cross-hand fingering is opt-in through an explicit `alt_fingers` token, for example:

```yaml
alt_fingers:
  - {ngram: [H], fingers: [LI], mode: allow}
  - {ngram: [Y], fingers: [LM], mode: allow}
```

Internally the tokens map to eight finger states ordered by `hand * 4 + finger`: `LP=0`, `LR=1`, `LM=2`, `LI=3`, `RP=4`, `RR=5`, `RM=6`, `RI=7`. A bare `P/R/M/I` token still means the physical key's own hand, so the shipped fingermap and existing rules keep their same-hand meaning. Cross-hand travel is priced from absolute key/finger positions, not from a hand-relative mirror, and cross-hand states remain unavailable unless a rule explicitly allows them. An explicit `allow` rule may name a finger outside the geometry reach map; the scorer then prices the reach with the normal travel weights instead of rejecting it. Same-hand pair terms use the fingering hand, so two cross-body keys typed by fingers of the same hand are still treated as a same-hand pair.

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

`compute` runs random anneals, keeps the best distinct layouts, and then
refines them with iterated local search (ILS). Useful flags:

- `--runs`, `--iters`, `--ils` — restart count, anneal length, and ILS rounds.
- `--ils-candidates` (default `4`) — how many top layouts advance to ILS.
  Candidates are chosen by letter placement diversity (at least four displaced
  letter keys), so layouts that differ only in punctuation are not treated as
  independent candidates.
- `--seed`, `--seed-layout`, `--seed-reference`, `--seed-all-references` —
  reproduce a run or add explicit starting layouts. Random starts remain the
  default, and references are not used as warm starts unless requested.
- `--polish-rounds`, `--polish-candidates` — optional final local polish.

The `hits` column counts independent starts that reached the same letter
placement, ignoring punctuation. Punctuation is placed canonically in the
final layout after the letter search, because punctuation positions do not
affect the primary score.

### Priority tiers

`priority_list` groups objective terms into ordered tiers. Earlier tiers receive a larger multiplier than later tiers, while omitted terms use their default weight. This makes it possible to prioritize, for example, low SFB/SFT and scissors over rolls without removing lower-priority terms from the objective.

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

### Conditions

Condition rules are evaluated from top to bottom; the first matching rule wins. `allow: false` forbids a matching pattern and `allow: true` permits it. Unless `reverse: true` is specified, a scissor rule matches both regular and reverse scissors.

`fingers` matches when either endpoint is in the listed set. Use `pairs` when the rule must match both endpoints exactly, for example `pairs: [[LR, LP], [RR, RP]]` for ring/pinky pairs in either direction. Pair entries must use explicit hand/finger tokens.

The shipped condition list is a built-in default and stays disabled because the fitted model already implements those native rules. Editing the list enables the conditions engine for the edited list; the built-in defaults are kept in `src/config.rs` (and `utils/config.py`) so a config that still matches them stays disabled.

## Analysis
> Details about shirolyzer's layout generation

The **shirolyzer** analyzer uses the same configuration, geometry, model, and fingering rules as the generator. It reports:

- bigram categories such as SFBs, scissors, and stretches;
- trigram patterns including rolls, redirects, alternation, true same-finger trigrams (Sft), and adjacent same-finger trigrams (Sfb/BadSfb lumped); repeated-character trigrams are also reported as an overlapping `repeat` diagnostic;
- when `redirect_tiers` is configured, per-tier and per-pattern redirect percentages plus `redirect_unclassified` for unlisted turns; the older `redirect`/`redirect_weak` categories remain as deprecated overlapping diagnostics;
- per-finger usage against the configured finger-balance targets;
- the pattern-model score, including priority-tier weighting.

`analyze` uses the most recent `compute` result when `--layout` is omitted. The same metrics drive primary-sacrifice enforcement during secondary optimization, so scores shown by the analyzer reflect the constraints the generator applied.

### Corpus provenance

`analyze` records a `corpus_provenance` block for the primary and secondary corpora. Each entry names the resolved path, corpus kind, configured weight, per-file SHA-1 and combined hash, parser rule id, normalization rule, and n-gram totals. The text report prints a compact summary; `--out-json` contains the full record. Provenance is observational and never changes a score.

`compare` can read an external comparison bundle with `--corpus-json FILE`. The file is a single JSON object with `letters`, `bigrams`, and `trigrams` maps from n-gram strings to counts. This is a comparison-only input: it does not replace the native parser, and the config corpora still drive generation and the shipped baseline.

---

<div align="center">

</> with <3 by [akai_hana](https://github.com/akai-hana)

</div>
