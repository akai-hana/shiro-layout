# Review of the computed `gwhts` layout

Date: 2026-11-07

> **W2 update:** the shortcut-constrained layout was adopted on 2026-11-07.
> The metrics below describe the superseded layout. See
> [W2 adoption](#w2-adoption-shortcut-constrained-layout) for the retained
> layout and its metrics.

Layout:

```text
g w h t s | m u e n q
l r d v ; | o a j p i
c x k f z | , . b y /
```

This review treats flexible, context-dependent fingering as the intended
typing model. Fixed-fingering results are diagnostic only and are not an
optimization target.

## Current metrics (original layout)

Shipped config (`shirolyzer/config/config.yaml`), primary corpus Shai:

| Metric | Value |
|--------|------:|
| Primary score | `4.261835` |
| Secondary score | `4.499064` |
| Conventional bigram SFB | `0.000%` |
| Trigram same-finger (`sfb_sft`) | `6.673%` |
| True SFT (`sft`) | `0.030%` |
| Adjacent SFB class (`sfb_tri`) | `6.643%` |
| Same-finger skipgrams (`sfs`) | `6.255%` (upper bound; see below) |
| Redirects | `3.847%` |
| Weak redirects | `1.038%` |
| Rolls | `46.406%` |
| Bigram alternation | `68.026%` |
| Mild scissor cost | `0.133` |
| Frequency-weighted strength | `0.172` |
| Audit | `166/166` |

Per-letter finger loads:

| Finger | Load |
|--------|-----:|
| LP | `7.276` |
| LR | `11.595` |
| LM | `14.930` |
| LI | `14.955` |
| RP | `8.491` |
| RR | `11.857` |
| RM | `16.341` |
| RI | `14.554` |

The layout uses a non-default finger on `18.10%` of letter strokes. `4.38%` of
letter strokes are cross-hand (`m` is `99.99%` LI on physical `Y`, `o` is
`24.01%` LI on physical `H`).

## Finding 1: the shortcut hand constraint is not fully applied

`shirolyzer/tomorrow.txt` states that `z, x, c, v, t, y` are restricted to the
left hand for one-handed Ctrl shortcuts. The config only contains:

```yaml
hand_restrictions:
  left: [z, x, c, v]
```

The computed layout leaves `y` on physical `.` (right hand), so Ctrl+Y is a
two-handed shortcut. `t` happens to be on the left, but it is not restricted.

`c` is on physical `Z`, which the fingermap assigns to LP (left pinky). A
left-Ctrl + C shortcut is therefore same-finger. This is not covered by a
hand-only restriction; it needs either a finger-aware shortcut constraint or
a deliberate layout choice.

A local `y<->f` swap puts `y` on the left index (`gwhtsmuenqlrdv;oajpicxkyz,.bf/`)
at primary `4.3357` (`+1.73%`); the swap also sends `f` to the right ring.
Simple `c` swaps are worse: `c<->v` costs `+1.75%` and makes Ctrl+V
same-finger; `c<->z` costs `+1.22%` and makes Ctrl+Z same-finger; `c<->x`
explodes under the hard-SFB constraint. A constrained recompute is the right
way to test whether the shortcut requirements can be met cheaply.

## Finding 2: `sfb_sft` is repeat-dominated

The exact adjacent-repeat share of the primary trigram corpus is `6.672916%`.
The analyzer reports `sfb_sft = 6.672916%` to the same precision. In other
words, every counted adjacent same-finger trigram is a repeated character;
there are no adjacent same-finger trigrams on distinct keys under flexible
fingering. `sft = 0.029684%` is exactly the `aaa` share, and `sfb_tri` is the
remaining adjacent repeats.

The same contamination affects `sfs`. Non-adjacent repeats are `3.395%` of
trigrams, and an unknown subset of them is counted as SFS because the first and
third keys are the same key and therefore the same finger. A repeat-free SFS
would be between `2.855%` and `6.255%`, not the reported `6.255%` without
qualification.

This does not change the fitted score, and the current budget intentionally
counts repeated-character trigrams. It does mean the headline "trigram
same-finger `6.673%`" must not be compared directly with community SFS/SFB
tables. Suggested response:

- keep the current diagnostic and budget;
- report repeat-free `sfb_tri`, `sft`, and `sfs` alongside them;
- state the repeat share in the analysis output and README.

## Finding 3: punctuation placement is feasibility-locked, not optimized

Punctuation is not scored; it is canonicalized after the search. The four dead
keys are physical `G`, `N`, `M`, and `/`, with `;` on `G`, `,` on `N`, `.` on
`M`, and `/` on `/`. Permuting punctuation among those keys changes nothing.
Moving a letter onto a dead key or punctuation onto a letter key usually
explodes the hard-SFB score, so the dead-key set is a feasibility constraint,
not an ergonomic choice.

Two practical gaps remain:

- apostrophe is not modeled; the xkb map leaves it on `AC11`/`LSGT` next to
  the right pinky `i`;
- shifted symbols are not scored. This is mostly inherited from QWERTY, so
  shortcut and shift-layer behavior is better preserved than a layout that
  rewrites the number row, but it is still invisible to the analyzer.

If prose punctuation matters, add `'` as an optional scored character or
report its estimated cost. Do not let punctuation optimization displace the
hard-SFB letter constraints.

## Finding 4: cross-hand alt fingering is a human-factors check

The model chooses cross-hand `m` on `Y` (`99.99%` LI) and cross-hand `o` on `H`
(`24.01%` LI). This is not a scoring bug: the flexible model prices travel and
the alternative wins. It is, however, a typing-reality check. If the guide is
not actually typed that way, the measured score overstates the layout.

Recommended response:

- document the cross-hand contexts in the layout README;
- verify in real typing that `m` with left index is comfortable;
- if it is not, change the guide or the alt rule, not the layout's alt-fingering
  model.

## Finding 5: fixed-fingering mode is not an objective

For the record, the current layout scores `7.785231` with conventional
fixed-fingering config, versus `5.300331` for the previous `cntgf` layout.
The fixed-mode load is heavily skewed (`LM = 21.20`, `LI = 3.91`).

This is expected: fixed mode discards the context-dependent fingers that the
layout was optimized to use. It should not be used as a regression gate, and
it should not be fixed by adding an alt-finger penalty that pushes the layout
back toward default fingering. The flexible model, with geometry-priced travel
and explicit alt rules, is the design target.

## Finding 6: documentation and comparison drift

- `references.toml` and `shirolyzer/README.md` previously recorded
  `4.292242`; the current shipped config produces `4.261835`. The stored value
  came from an intermediate config state.
- `compare` under the shipped hard-SFB config is not a fair reference
  comparison: layouts with unavoidable SFBs receive enormous scores. Use
  `config.fixed.yaml` or a permissive config for cross-layout comparison, or
  add a warning to `compare`.
- `just check` passes (`99` unit tests, `1` baseline, `10` command tests, `6`
  parity tests), but the baseline and reference tests use the separate
  reference config, so they do not catch shipped-config score drift.

## Ranked objectives

1. **Apply the shortcut constraint.** Add `t` and `y` to the left-hand
   restriction, rerun the constrained compute, and add a finger-level check for
   `c` versus left Ctrl. Prefer a constrained search over a forced swap.
2. **Make the metrics honest.** Add repeat-free SFB/SFT/SFS diagnostics and
   label `sfb_sft` as including repeated characters. Do not change the fitted
   score or the budget unless a separate experiment justifies it.
3. **Keep documentation in sync.** Refresh `references.toml` and README scores
   after the final config is fixed. Add a test or command that recomputes the
   stored shiro score under the shipped config.
4. **Make `compare` config-aware.** Warn when `sfb_impossible` is active or
   automatically use a permissive comparison config for references.
5. **Add optional punctuation coverage.** Score or report apostrophe and the
   shift layer if prose/programming punctuation is a real use case.
6. **Run the SFS-weighted re-anneal.** This is the remaining high-value search
   experiment: keep the hard letter constraints and flexible fingering, but
   give repeat-free SFS a first-class term and see whether the layout moves.

## Explicit non-objectives

- Do not optimize for the fixed-fingering score.
- Do not penalize context-dependent alt fingering to make fixed mode look
  better.
- Do not reduce finger travel by discarding the alt-fingering model; travel is
  already priced in the flexible model.
- Do not treat `sfb_sft = 6.673%` as a conventional SFS/SFB comparison.


## W2 adoption: shortcut-constrained layout

`hand_restrictions.left` now lists `z, x, c, v, t, y`. A constrained
`compute` run (28 starts, 200k iterations, 8 ILS) produced the same best layout
across three seeds, so it was adopted without a hand-forced swap:

```text
b t h c r | m d i s j
n p l y ; | e o u g a
v k x z , | q . w f /
```

Shipped config (`shirolyzer/config/config.yaml`), primary corpus Shai:

| Metric | Value |
|--------|------:|
| Primary score | `4.264577` |
| Secondary score | `4.464993` |
| Conventional bigram SFB | `0.000%` |
| Legacy trigram same-finger (`sfb_sft`) | `6.673%` |
| True SFT (`sft_true`) | `0.000%` |
| Legacy SFT (`sft`) | `0.030%` |
| Adjacent SFB class (`sfb_tri`) | `6.643%` |
| True SFS (`sfs_true`) | `2.306%` |
| Legacy skipgrams (`sfs`) | `5.701%` (upper bound; includes repeats) |
| Redirects | `3.956%` |
| Weak redirects | `0.683%` |
| Rolls | `47.281%` |
| Bigram alternation | `65.0%` |
| Mild scissor cost | `0.179` |
| Frequency-weighted strength | `0.176` |

Per-letter finger loads:

| Finger | Load |
|--------|-----:|
| LP | `8.00` |
| LR | `13.57` |
| LM | `13.60` |
| LI | `13.94` |
| RP | `7.87` |
| RR | `10.53` |
| RM | `14.79` |
| RI | `17.70` |

Shortcut report under the shipped config:

| Letter | Key | Chosen | Default | Same hand | Same finger |
|--------|-----|--------|---------|-----------|-------------|
| `z` | V | LI (`86.8%`) | LI | yes | no |
| `x` | C | LI (`88.0%`) | LI | yes | no |
| `c` | R | LM (`77.2%`) | LM | yes | no |
| `v` | Z | LP (`87.4%`) | LP | yes | yes |
| `t` | W | LR (`98.9%`) | LR | yes | no |
| `y` | F | LI (`86.1%`) | LI | yes | no |

All six shortcut letters are on the left hand, and Ctrl+C is no longer
same-finger. Ctrl+V remains same-finger because `v` is on the left pinky; the
stated requirement is hand-only, so this collision is accepted and documented
here.

Cross-hand alt usage moved to `m` (`99.48%` LI on physical `Y`) and `e`
(`30.83%` LI on physical `H`). The total cross-hand share is `6.20%`, and
non-default fingers are used on `18.81%` of letter strokes. The fixed-fingering
diagnostic is unchanged and remains a non-objective.
