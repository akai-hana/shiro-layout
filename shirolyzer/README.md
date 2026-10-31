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
- Alt-finger ngram rules that override the fingermap for specific physical-key sequences.
- Conditional preferences/restrictions for scissors, spread, separation, and crossing.

**Tuning**:

- Primary and secondary corpora, with weight-specific blending.
- Primary sacrifice limits, expressed as score and/or per-metric percentages.
- Optional per-hand character restrictions (`hand_restrictions`), so selected characters are only placed on one hand during generation.
- Priority tiers for selected objective terms.

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

Percentages are relative to the best primary layout. Lower-is-better metrics (`sfbs`, `scissors`, `redirects`, `finger_balance`) may increase by the given percentage; higher-is-better metrics (`rolls`, `alternation`) may decrease by it. `sfb`, `sfts`, and `sfbs` are aliases for the combined SFB/SFT budget. `score` is optional only when every metric is specified.

### Alt-fingers & Context

The generator takes user-defined alt-fingers into account as optimization criteria. These override the selected fingermap.

Alts are defined from physical key positions rather than characters, so the generator can apply them to the evolving letter arrangement:

```yaml
alt_fingers:
  - {ngram: [F, J], fingers: [LP, RI], mode: allow}
  - {ngram: [S, T, R], fingers: [LR, LM, LI], mode: block}
```

- Matching is exact and ordered. Longer ngrams override shorter ngrams.
- `allow` restricts an ngram to the listed fingering; `block` forbids that exact fingering while leaving other fingerings available.
- Listed fingers are mandatory. An explicit rule never silently falls back to the default fingermap.
- When the same ngram is defined more than once, earlier definitions are preferred; later definitions act as fallbacks.
- Context-key defaults are listed first, followed by other fingers on the same hand as fallbacks.

### Priority tiers

`priority_list` groups objective terms into ordered tiers. Earlier tiers receive a larger multiplier than later tiers, while omitted terms use their default weight. This makes it possible to prioritize, for example, low SFB/SFT and scissors over rolls without removing lower-priority terms from the objective.

### Conditions

Condition rules are evaluated from top to bottom; the first matching rule wins. `allow: false` forbids a matching pattern and `allow: true` permits it. Unless `reverse: true` is specified, a scissor rule matches both regular and reverse scissors.

`fingers` matches when either endpoint is in the listed set. Use `pairs` when the rule must match both endpoints exactly, for example `pairs: [[LR, LP], [RR, RP]]` for ring/pinky pairs in either direction. Pair entries must use explicit hand/finger tokens.

The shipped condition list is a built-in default and stays disabled because the fitted model already implements those native rules. Editing the list enables the conditions engine for the edited list; the built-in defaults are kept in `src/config.rs` (and `utils/config.py`) so a config that still matches them stays disabled.

## Analysis
> Details about shirolyzer's layout generation

The **shirolyzer** analyzer uses the same configuration, geometry, model, and fingering rules as the generator. It reports:

- bigram categories such as SFBs, scissors, and stretches;
- trigram patterns including rolls, redirects, alternation, and SFB trigrams;
- per-finger usage against the configured finger-balance targets;
- the pattern-model score, including priority-tier weighting.

`analyze` uses the most recent `compute` result when `--layout` is omitted. The same metrics drive primary-sacrifice enforcement during secondary optimization, so scores shown by the analyzer reflect the constraints the generator applied.

---

<div align="center">

</> with <3 by [akai_hana](https://github.com/akai-hana)

</div>
