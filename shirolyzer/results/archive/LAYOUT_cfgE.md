# The Layout (`cfgE_semiQ`) — Shiro【白】

> **Historical guide (2026-10-26):** This documents the shipped
> `cfgE_semiQ` layout under the earlier Norvig/root model. A newer
> provisional Shai-primary winner exists; see `results/LAYOUT.md` and
> `analysis/REPORT_round8.md`. The scores below are not comparable to the
> calibrated Shai-primary scores.

Shiro's primary configuration. Product of the investigation in
`findings/investigation_report.md`, optimized for the README rules
(arched rest position `AWEF/JIO;`, extensive context-aware alt-fingering,
scissor intolerance).

## Grid (30 main keys, QWERTY names = physical positions)

```
        L hand            R hand
top :   ; a i o j        v l n r w
home:   h f p e y        g t c m s
bot :   b x q u z        k d , . /

rest:   A W E F          J I O ;
        pinky ring mid idx | idx mid ring pinky
```

30-char string (KEY_ORDER): `;aiojvlnrwhfpeygtcmsbxquzkd,./`

## Punctuation, space, ISO key

| char | key | rationale |
|---|---|---|
| `,` | comma key (bot col8, right ring) | natural position, rule A.9 alts apply |
| `.` | period key (bot col9, right pinky) | natural position |
| `/` | slash key (bot col10, right pinky) | natural position |
| `;` | **Q key (top col1, left pinky corner)** | frees right-pinky home for `s`; `s;` becomes cross-hand alternation (best prose+code punctuation bigram after `.;`/`, `); low 0.13% usage makes the corner fine |
| `'` | **ISO-102 key** (outside the 30-grid, right pinky). ANSI fallback: the `'` key right of `;` | README A.10 hint + keeps grid clean; `'s/s'/n't` all become pinky↔pinky-home rolls or alternation |
| space | **right thumb** | analysis showed left/right thumb essentially tied (49.5% vs 50.5% alternation into/out of space); right is conventional |

Alternative punctuation config (`cfgG_semiP`, near-identical letter
placement): `;` on the P key (top-right corner, Colemak/Canary style) —
scores 4.401 vs 4.388 (tie within noise), but makes `s;` a same-finger
pinky pair. Prefer `cfgE` unless you strongly want community-conventional
semicolon placement.

## Design at a glance

- **All five vowels on the left hand**: `a`@W (ring rest), `i`@E (middle
  rest top), `o`@R (middle-top, mostly index-alted), `e`@F (index rest),
  `u`@V (index-bottom). Right hand carries `n r l s m t c d k w v g`.
- High-frequency letters sit on or beside rest keys: `h`@A (pinky rest),
  `s`@; (pinky rest), `e`@F, `a`@W, `i`@E, `t`@J (index rest), `c`@K
  (middle rest), `m`@L, `r`@O, `n`@I.
- Low-priority corners hold low-frequency letters/punct: `b`@Z, `x`@X,
  `q`@C, `z`@B, `;`@Q-corner, `,./` bottom-right.
- `y`@G stays on the left index (cross-hand into right-hand vowels).

## Key statistics (Norvig big.txt corpus, this investigation's model)

| metric | this layout | best reference* | note |
|---|---|---|---|
| objective (trigram DP + balance, ↓btr) | **4.388** | 5.576 (Sturdy) | ~21% lower than any reference |
| SFB under alt-fingering | **0.02%** | 0.01–0.20% | essentially eliminated |
| SFB, default fingering only | 3.53% | 1.47–3.74% | mid-pack; nearly all removable via prescribed alts (unavoidable 0.02%) |
| weak redirects (no index) | **0.55%** | 0.28–2.15% | far below Colemak-DH (2.06%) |
| total redirects (trigram) | **3.4%** | 2.6–10.3% | |
| one-hand trigrams | **0.7%** | 1.0–2.1% | lowest |
| rolls : alternation (trigram) | 41.4 : 40.1 | — | roll-heavy without roll-excess |
| scissors HSB (prefer-avoid IM) | 0.69% | 0.36–1.92% | priced mildly; main: `po`, `op`, `cl` |
| scissors MR (intolerant class) | 0.72% | 0.27–0.91% | mostly rest-position pairs (`pa`, `ap`, `cr`, `rc`) where both fingers park in their natural columns — see interpretation note in the report |
| scissors RP (intolerant) | **0.00%** | 0.00–0.12% | |
| finger usage L/R | 51.9 / 48.1% | — | index≈16–17% each, pinky≈7% each |
| corpus-robustness | wins big.txt; 2nd by 0.01 on OpenSubtitles/Gutenberg (tie); **#1 on shai-iweb** (4.4404, 12-layout table, +1.25 over Graphite) | | cross-validated on 4 corpora |
| weight-robustness | **#1 in 17/18 weight-perturbation scenarios** | | `results/robustness.txt` |

\* references = Gallium, Graphite, Colemak-DH, Canary, Sturdy, Semimak,
Colemak, QWERTY under the same model; full table in
`results/final_comparison.json`.

External check (oxeylyzer, **standard** fingering, its own corpus):
SFB 4.11%, redirects 4.59% (very low), bad-redirects 0.95%, trigram
rolls 35.1%, alternation 44.3%, score 0.100 (Canary 0.427, Colemak-DH
0.287 on the same tool). The layout is **not** designed for standard
fingering — its SFB number there is the price of the arched model;
under the README fingering it is 3.53% default / 0.02% with alts.

## Fingering guide (context-optimal, from trigram DP)

Default fingers per README; the DP says these alts actually occur:

| letter | key | default | typical alts (contexts) |
|---|---|---|---|
| o | R | M | **I ~85%** — `ion tio for con not` (index straight up from F rest is cheaper than middle diagonal) |
| l | U | M | **I ~88%** — `all ill ell lly lle` |
| q | C | I | **M ~72%** — `qui equ qua req` (avoids `qu` index SFB → M-I roll, README A.8 pattern) |
| j | T | M | I ~37% — `joi inj adj joy` |
| c | K | M | I ~14% — `nce inc enc anc nch` |
| p | D | M | I ~13% — `app pie pin pai spi` |
| v | Y | M | I ~97% — `ver eve ove ive ave` (mirror-rule alt) |
| f | S | R | M ~8% `fac aft fat far`; I ~2% `ifi fai` |
| m | L | R | M ~5% `orm arm erm rma rme` |
| x | X | R | M ~13% `exa xam tax` (README XC/CX) |
| u | V | I | M ~3% `que sue uen` |
| e | F | I | M ~1% `hey yes ize` |
| i | E | M | R ~1% `qui iou joi` (README UI/IU) |
| a,h,s,r,n,t,d,w,y,g,k,b,z | rest/home keys | as README | essentially 100% default |

Rule Y/Z are honored by construction: the scorer's trigram DP only takes
an alt when the *whole trigram* (including the third key's finger) gets
cheaper — e.g. `ion` keeps `o` index because nothing downstream needs
middle; `erf`-type sequences fall back to ring-middle-index as the README
prescribes.

## Known trade-offs

1. **`b`@Z** (1.5% letter on the pinky-bottom avoid-key). The annealer
   preferred this over `b`@B (which created `ub/bu/be/eb…` index SFBs;
   verified worse by +0.08). `z` remains at `B` (rarest letter on an
   index key — QWERTY-adjacent muscle memory for `z` survives).
2. **`gt/tg`, `kg/gk`, `hb/bh`** are small unavoidable same-finger pairs
   (total ≈0.015% of bigrams) — index-column clustering `g@H t@J k@N
   d@M` / pinky column `h@A b@Z` is the cost of the vowel-left,
   rest-heavy arrangement.
3. **`dg/uz`-class SFBs** (~0.03%) are "removable but expensive" via the
   documented M-alts (cost ≈ SFB anyway); use default fingering unless
   rolling.
4. **Standard-fingering hostility** — do not evaluate this layout with
   tools that assume traditional fingering without reading the arched
   model first; SFB numbers will look bad by design.
5. **Punctuation concentration on the right pinky** — `.` and `/` both sit
   on it (plus `s` on home), carrying ~1.2% of keystrokes; `s.` is a
   same-finger pair under the letters-only objective, which never sees
   punct mass. B6 priced punct in (2.29% of keystrokes): a joint re-search
   finds a stable layout that relocates `.` (−0.087 punct-axis) but costs
   +0.070 on the English objective — so cfgE stays primary and the fix is
   shipped as the opt-in `cfgE-punct` variant below
   (`results/b6_punct.txt`).

## Variants

| variant | string | EN score | punct-priced | when to use |
|---|---|---|---|---|
| **cfgE** (primary, installed) | `;aiojvlnrwhfpeygtcmsbxquzkd,./` | **4.3877** | 4.5526 | default — best English, README design rules |
| cfgG | `faiojwlrn;hgpeyvtcmsbqxuzkd,./` | 4.4010 | 4.5920 | tie within noise; conventional `;` on P (makes `s;` same-finger) |
| cfgE-punct (B6) | `.aiozvlnrbhgpeymtcwsfjxu/kd,;q` | 4.4573 | **4.4652** | if you type heavy punctuation/code and accept `.` on Q; kills the `s.` pinky pair |
| hybrid-35 (B8) | `;aemjwcioksfgnhptubrqzylxvd,./` | 4.5030 | — | EN+ES bilingual side layout: −17% Spanish score for +0.12 EN; beats every community layout on both axes |
| hybrid-80 (B8) | `;eiowgcnrvszpafmtbhljkyuxqd,./` | 4.9110 | — | ES-heavy typing (Spanish dominant); also beats all refs both axes |

Punct-priced = B6 objective (trigram+balance with punct mass included,
`corpus/punct/`). All variants use the same 30-key alphabet; Spanish ñ/accents
are outside the model. Frontier details: `results/hybrid_pareto.txt`.

## Files

- `results/analysis_winner.txt` — full diagnostic dump
- `results/analysis.json` — machine-readable metrics + per-letter alt guide
- `results/final_comparison.json` — reference comparison table
- `results/letter_sensitivity.txt` — pairwise swap deltas (letter immovability)
- `results/spanish_analysis.txt` — Spanish-vs-English extension (scores, tiers, pinned search; corpus in `corpus/spanish/`)
- `results/shai_comparison.txt` — A1 cross-corpus check (corpus in `corpus/shai/`)
- `results/keysolve_report.txt` — A2/A3 keysolve-compatible metrics + percentile bands
- `results/sfs_ladder.txt` — A3 SFS distance ladder + SFB denominator cross-checks
- `results/robustness.txt` — A4 18-scenario weight sweep
- `results/b6_punct.txt` / `results/b6_ablation.json` — B6 punctuation-priced search
- `results/hybrid_pareto.txt` — B8 EN+ES blended frontier
- `results/search_batch2.log/.json` — annealing run (20 restarts × 100k iters × 4 punct configs)
- `analysis/` — model, scorer, search, metrics (reproducible: `python3 analyze_layout.py ";aiojvlnrwhfpeygtcmsbxquzkd,./"`)
