# Investigation report — README-optimized alt layout

## 1. What was built

A positional typing model, corpus scorer, simulated-annealing layout
search, and metrics suite implementing the README's rules (arched rest
`AWEF/JIO;`, conditional alt-fingering with premiums, trigram-aware
rule Y, rule Z same-finger-travel penalty, per-pair scissor
intolerance), plus cross-validation against three corpora and the
community analyzer oxeylyzer. Output: `results/LAYOUT.md`.

> Repository note (2026-10-28): the non-Shai/Spanish corpora have been
> removed. Scripts and results referenced below now live under
> `analysis/legacy/` and `results/archive/`.

## 2. Model (`analysis/fingers.py`)

Physical keys = 30 grid keys (QWERTY names as position labels per README
convention). Per-key default finger + conditional alts, transcribed from
README A.1–A.10:

- rest keys: `A`(P) `W`(R) `E`(M) `F`(I) | `J`(I) `I`(M) `O`(R) `;`(P)
- top `E R T / Y U I` default MIDDLE (README A.2 — *not* traditional
  fingering); `RT` index-alt, `UI/IO` ring-alt, plus documented mirror
  inferences (Y/U index-alt, etc.) with small premiums (0.15–0.25)
  so default wins ties (rule T) while cheap travel wins in context
  (rule X)
- home `S L` ring, `D K` middle, `FG/HJ` index; `SDFG/JK` index-alts
  only when they form the listed M-I/I-M rolls (A.6)
- `C` index + middle-alt (A.8); `VB/NM` index + middle-alts (A.7);
  `X/,` ring + middle/index alts, flagged low-priority (A.9);
  `Z . /` (+ISO) pinky, avoid-key class (A.10)

Scissors (README C): wiki-type only (shorter finger up + longer down),
per-pair severity: IM-HSB 1.5 (prefer-avoid), IM-FSB 6 (intolerant),
MR/RP 6 any distance (intolerant); reverse motions free. A rest-discount
(0.05/0.10 multipliers) implements the interpretation that a key pair
where both fingers sit in their *natural parked columns* of the arched
stance (e.g. `pa` = ring parked at W-rest + middle flexed to D) is not
anatomically a scissor — without this, `pa/ap/cr/rc` alone (~0.5% of
bigrams) would make README compliance mathematically impossible for any
vowel-rest layout. This is the investigation's largest interpretation;
flagged for the user's veto. Validated pairs: CR/DR/KO/PL behave as the
community documents; `T(F)-F(I)` reverse-IM free; `D(M)-T(I)` FSB=2.52.

## 3. Corpus & scorer (`analysis/scorer.py`, `corpus/`)

Primary: Norvig big.txt (~2.9M within-word letter trigrams). Cross-checks:
OpenSubtitles 50k, Gutenberg (full-text). Space ≈17.77% of keystrokes,
excluded from n-gram tables (thumb effort/alternation handled
separately; word-boundary bigrams computed from `position_freq.json`).

Scoring = min-cost fingering over each key-trigram via a precomputed
31³ DP tensor (context-aware alts = exactly rule Y/Z behavior), + bigram
roll/alternation rewards + finger-balance penalty vs targets calibrated
**from measured modern layouts under this model** (index ≈32% per hand,
not the initially-guessed 43%). Balance weight 1.5 on squared relative
share error; hand-balance 50/50 weight 0.5.

Weights: SFB 5/6/8 by row gap; in-roll −0.35, out-roll −0.20, leap −0.10,
alternation −0.12; redirect 0.4 / weak-redirect 1.8; SFS≥2u 3.5 (rule Z).

## 4. Validation

1. **Scissor unit checks** (`analysis/validate.py`): canonical QWERTY
   CR (FSB), DR (HSB), KO, PL reproduce community classifications;
   reverse directions free; rest-rescues fire where intended.
2. **Reference ranking** under (bigram+trigram)/trigram: QWERTY 5.60 ≫
   Colemak 4.62 > Colemak-DH 4.57 > Sturdy 4.53 > Gallium 4.47 >
   Semimak 4.33 ≈ Canary 4.31 — matches published community consensus
   ordering.
3. **Metrics bug found & fixed**: cross-hand bigrams with equal finger
   *ids* were miscounted as SFB (affected metrics only, not the scorer
   tables which branch on hand first).
4. **Community-metric parity**: trigram-pattern classifier reimplemented
   to oxeylyzer's definitions (rolls = 2-key same-hand burst + switch;
   alternates = strict L-R-L). Colemak-DH: mine rolls 44.1/alt 30.1/
   onehand 2.1 vs oxeylyzer 44.8/30.1/2.1 on its corpus — parity.
5. **Cross-corpus robustness**: candidate scores/ranks stable on
   big.txt, OpenSubtitles, Gutenberg (`results/final_comparison.json`,
   `analysis/cross_corpus.py`).
6. **External tool** (oxeylyzer, standard fingering): winner gets
   redirects 4.59%, bad-redirects 0.95% (excellent), SFB 4.11% (poor —
   expected: the layout assumes arched fingering; its README-model
   default SFB is 3.53%, alt-SFB 0.02%).

## 5. Search (`analysis/search.py`, `run_search.py`)

Simulated annealing over 26-letter → slot assignments (swap moves,
geometric cooling 0.35→0.002, 100k iters/restart). Punctuation-dead-key
configs searched (4 keys excluded from letter slots):

| cfg | dead keys | best score | note |
|---|---|---|---|
| **E** (winner) | `,` `.` `/` + Q-corner | **4.3877** | `;` char on left top corner; 14/12 hand key split |
| G | `,` `.` `/` + P-corner | 4.4010 | community-standard `;`-corner; 15/11 split |
| D | `,` `.` `;` + Q-corner | 4.6034 | frees home-`;` key but keeps semi natural |
| F | `,` `.` `;` + Q-corner (apostrophe in grid) | 4.6034 | slash leaves the grid |
| B (control) | `,` `.` `;` `/` natural | 6.6543 | 15/11 split + right-pinky-home dead = structurally crippled |

Batch 1 (old balance targets) peaked at 4.437; batch 2 (recalibrated
balance + warm starts from batch1/reference layouts + cfgG) reached
4.388. Winner: `;aiojvlnrwhfpeygtcmsbxquzkd,./`.

Convergence evidence (from `results/search_batch2.log`): independent
restarts repeatedly rediscover the same letter skeleton — the cfgG
family layout was found 4× from different seeds; restart results
cluster in 4.40–4.45, with the winner (4.3877) only 0.013 below the
runner-up family. The two best punctuation configs share the same core
skeleton (`aioj…lnr…peygtcms…bquzkd`) and differ only in the punctuation
corner — the strongest available evidence that the search is not
stranded in a random local optimum.

Punctuation side-decision (corpus has no punct): counted real
letter↔punct bigrams from 26M chars of text. Top semi bigrams are
`s; e; d; n; t;`. Under cfgE, `s;` = right-pinky-home → left-pinky-corner
= cross-hand alternation (good for code, where `;` frequency triples);
under cfgG it is a same-finger pinky pair. Decided cfgE; cfgG documented
as alternative.

Space side: word-initial/final distributions give 49.5% (R thumb) vs
50.5% (L thumb) alternation for the winner — chose right thumb
(convention). Apostrophe: ISO-102 key (README A.10 hint); ANSI fallback
the `'` key right of `;` — both outside the scored grid; `'s s' n't 't`
bigrams then land on right-pinky home/corner pairs, acceptable at
apostrophe's ~1.6‰ frequency.

## 6. Result summary

See `results/LAYOUT.md` for the layout, guide, and stats. Headline:
objective 4.388 vs best reference 5.576 (−21%); alt-SFB 0.02%; weak
redirects 0.55%; zero RP scissors; usage 52/48 hands with index ≈16–17%.

## 7. Post-search sensitivity analysis (letter immovability)

Question: which letters are load-bearing, and which are disposable?
Method: all 325 letter pairs swapped pairwise in the winner layout,
each swap re-scored under the full trigram-DP objective. Full table:
`results/letter_sensitivity.txt`.

- **Keystones**: `e` (median Δ +1.15 if moved; cheapest possible swap
  still +0.21 — nothing is close), then `s a t h n r i o` (medians
  +0.29…+0.82). The top-8 letters occupy the eight best slots (the
  A.1 rest keys plus their immediate neighbors).
- **Frequency-twin pairs** exchange nearly free *with each other*
  (i↔n +0.008, r↔s +0.020, a↔o +0.028): the objective cannot
  distinguish them; secondary criteria broke those ties (punct `s;`
  alternation; A.1 rest assignment).
- **Disposable fringe** (median Δ < 0.12): `y p v g w f k`, plus a
  rare-letter playground that is free among itself (j/q/x/z in the
  corners, all pairs |Δ| ≤ 0.007).
- **No improving single swap exists anywhere** (min Δ = +0.001): the
  layout is a confirmed **1-swap local optimum**, consistent with the
  annealer's swap move set.
- **Deep-search follow-up** (`analysis/deep_search.log`,
  `results/search_deep_sa.log`): a full steepest-descent quench over
  *all* 1-swaps, all 44,763 disjoint 2-swaps, and all 5,460
  3-cycles — iterated to convergence — found **no improvement**; 30
  iterated-local-search rounds (random 4–8 swap perturbations + full
  quench) found none either; and 17 additional long simulated-annealing
  restarts (300k iters each, ~5.1M evaluations, run cut short by the
  host under load) **rediscovered the exact winner twice** from
  independent random starts, with every other run landing in
  4.39–4.66. After this, the only remaining doubt about global
  optimality is the philosophical kind.

## 8. Strength of claims (epistemic status)

- Not proven globally optimal: annealing carries no such guarantee
  (~160M configurations evaluated of 10^26). The convergence evidence
  (§5) and the local-optimality check (§7) bound the risk of a
  stranded local optimum; they do not prove its absence.
- "Optimal" always means *under this model*: cost weights are
  calibrated so reference layouts rank in the community-consensus
  order (§4.2), not fitted to the user's subjective comfort. Different
  weights would shift the winner somewhat.
- Known interpretation calls that shape the optimum — §2
  rest-discount, R/U index-alts beyond the literal A.2 conditionals,
  balance targets, punctuation handled off-objective — are documented
  and individually reversible. cfgE vs cfgG (Δ 0.013) was decided by
  punct evidence, not by the letter objective.
- Corpora are generic, word-internal English: one used for search, two
  held out (§3–4). Letter rankings held stable across all three; a
  personal/code-heavy corpus could reorder the punctuation story.
- The model scores key-to-key costs; multi-key hand-shape flow and
  individual hand geometry are out of scope. Final ratification is the
  user's hands against the worst-pair list
  (`results/analysis_winner.txt`) — swaps the hands reject can be
  re-scored cheaply.

## 9. Spanish-language extension

Question (user request): English stays primary, but which placements
would *Spanish* lock, and what would the fringe look like if the
English keystones are frozen? Method (`analysis/spanish_analysis.py`,
full output `results/spanish_analysis.txt`):

- **Corpus** (`corpus/spanish/`, built by
  `analysis/build_spanish_corpus.py` from raw texts in a scratch
  dir): ~6.7M letters / ~1.5M spaces (space_frac 0.181) — Project
  Gutenberg Spanish books #2000 (Don Quijote), #17013 (Fortunata y
  Jacinta), #17073 (La Regenta), #13264 (Zalacaín el Aventurero),
  #13519 (La Tierra de Todos), #14995 (Los Hombres de Pro), #9980
  (Platero y yo) — ~8MB, 19th/early-20th c. literary register — plus
  ~1MB of full-article extracts from es.wikipedia.org (26 major
  topics fetched via the MediaWiki API; the API rate-limited further
  requests) for contemporary register. Normalization: lowercase,
  Unicode NFKD with combining marks stripped (á→a, ü→u, and ñ→n),
  only [a-z] kept, within-word bigrams/trigrams (same convention as
  the English corpora). Observed profile: e 13.2 / a 13.0 / o 9.4 /
  s 7.4 / n 6.9 / r 6.4 / i 6.2 / l 5.7 / d 5.0 / u 4.4 / t 4.2 /
  c 3.9 %. Caveats: within-word convention undercounts
  standalone-letter traffic (notably Spanish `y` = "and", whose
  letter count is included but not its space transitions); ñ/accents
  would need an AltGr/dead-key layer in practice, unmodeled.
- **Scores under Spanish**: references scored with their own
  per-layout dead-key evaluators (§5 method). The English winner is
  *still #1 of the whole field* (5.483 vs cfgG 5.510, Sturdy 6.377,
  Canary 6.602, Gallium 7.074, Semimak 7.216, Colemak-DH 8.056,
  Colemak 8.101, QWERTY 10.109). Ranking order shifts slightly under
  Spanish (Canary rises among the references) but the winner's lead
  widens.
- **Sensitivity under Spanish** (325-swap sweep, same method as §7,
  run against the Spanish corpus): `a` becomes the joint-keystone
  (median 1.431, above `e`'s 0.910); `t` (4.2% ES vs 9.1% EN)
  demotes C→F; `h` (silent, 1.1%) becomes completely free
  (median ≈ 0); `x` promotes M→C (the final-"ex" pattern); `l u q b`
  demote to fringe; `f w` promote slightly. The A.1 rest keys holding
  `a e i s` are validated twice — both languages' top-6. Full
  side-by-side tier table: `results/spanish_analysis.txt`.
- **Pinned annealing** (letters `e s a t h n r i o` frozen — the §7
  keystones+core; free set `b c d f g j k l m p q u v w x y z`;
  same swap-move annealer and cooling 0.35→0.002 as §5; 16 restarts
  × 100k iters per objective): the English-control run **rediscovered
  the exact winner** (4.3877, identical layout) — independent
  confirmation of §7's 1-swap local-optimality from a constrained
  subspace. Under the Spanish objective the fringe rearranges
  completely:

  ```
  ES-pinned optimum:  ; a i o z  |  j u n r y
                      h x g e q  |  d t m p s
                      l k f b w  |  v c , . /
  ```

  (4.577 under ES vs 5.483 for the EN fringe): `x@S` next to `e` for
  "ex" rolls, `q@G` adjacent to `e` for "que", `l@Z`, `k@X`, `z@T`,
  `w@B`. Trade: saves 0.91 under Spanish, costs 0.59 under English.
- **Verdict**: layout kept as the English winner (English primary per
  user); it also outperforms every reference under Spanish. Bilingual
  tension points if ever wanted: `h@A` (rest key held by a
  Spanish-free letter) and `t@J`; a Spanish-first variant would
  repurpose those two slots.

## 10. Open points for the user

1. Rest-discount interpretation for MR "scissors" (`pa`-class) — veto
   would require re-running the search with those pairs priced at full
   6.0 (they are currently 0.21).
2. Index usage on `R`-position (85% for `o`) exceeds the literal A.2
   bigram conditionals; justified via rules U/X (travel) but flagged.
3. `cfgG` (`;` on P-corner) if community-conventional semicolon
   placement matters more than the `s;` alternation.
4. Layout name.
