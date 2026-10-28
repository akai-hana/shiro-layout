# Analyzer landscape: 12 community tools vs. our pipeline

Part of the cfgE_semiQ investigation — see `investigation_report.md` (methodology),
`akl_recommendations_research.md` (AKL recommendations), `../results/LAYOUT.md` (layout).

Status: complete. All code-level claims below were read from source (file:line given);
all numbers were produced by running the tools (commands in §9).

> Repository note (2026-10-28): the non-Shai/Spanish corpora have been
> removed. Scripts and results referenced below now live under
> `analysis/legacy/` and `results/archive/`.

---

## 1. Scope and method

Twelve analyzer repos cloned and studied under `/tmp/opencode/analyzers/` (ephemeral —
re-clone via §9 if needed):

| Repo | URL |
|---|---|
| genkey | https://github.com/semilin/genkey |
| oxeylyzer | https://github.com/O-X-E-Y/oxeylyzer |
| cmini | https://github.com/Apsu/cmini |
| 200-analyzer | https://github.com/Apsu/200-analyzer |
| Keyrita | https://github.com/youngbrycecode/Keyrita |
| keycraft | https://github.com/rbscholtus/keycraft |
| algae | https://github.com/ExternalTime/algae |
| mana | https://github.com/Zakkkk/mana |
| cyanophage | https://github.com/cyanophage/cyanophage.github.io |
| keysolve-web | https://github.com/va1orance/keysolve-web |
| layouts-playground | https://github.com/O-X-E-Y/layouts |
| keyboard-layout-battles | https://github.com/malfple/keyboard-layout-battles |

Method:

1. Source reading of every repo (scoring model, weights, fingering model, corpora,
   search/optimizer, percentile machinery) — not README claims.
2. Execution where runnable: **genkey** (7 layouts), **keycraft** (view/analyse/rank,
   n=151 layouts), **200-analyzer** (a200 view), **oxeylyzer** (prior session), plus
   keysolve's `percentiles.json` extraction.
3. Not executed (code-only): cmini (Discord bot infra), Keyrita (Windows/WPF + native
   DLL), mana (Bun runtime not present), algae, cyanophage & keysolve-web (browser apps;
   data extracted directly), layouts-playground, keyboard-layout-battles.
4. Our pipeline for comparison: `analysis/{scorer,fingers,search,metrics}.py` —
   trigram-DP cost with context-dependent alternate fingering, balance inside the
   objective, annealer + quench/ILS floor proof.

---

## 2. Inventory

| Tool | Lang | Kind | Composite score? | Search? | Corpora shipped |
|---|---|---|---|---|---|
| genkey | Go | finger-speed + metric soup | yes (KPS Score) | generate (random population → greedy full improve → selection, `generate.go:120-175`) | shai-iweb, tr |
| oxeylyzer | Rust | metric soup + trigram patterns | yes (integer weights) | no (analyze only) | shai.json (mono/bi/skip1-3/tri, precomputed) |
| cmini | Python | Discord bot, triple-class lookup | no (user-driven sort) | no | 18 (shai, monkeyracer, mt-quotes, gutenberg, reddit, akl, dutch, french, keymash, wordle, ...) |
| 200-analyzer | Python | trigram classes + custom sort weights | custom (sort-weighted) | no | monkeytype-* wordlists |
| Keyrita | C#/WPF+C++ | measurement graph, interactive editor | yes (LayoutScore) | swap-event incremental | user text files (KLC import) |
| keycraft | Go | metric soup + loads + trigram classes | yes (normalized rank) | yes (BLS + generator) | default, monkeyracer, reddit_small, shai |
| algae | Rust | bigram weight table | none (prefilter/generator) | yes (exhaustive branch-and-bound) | user corpus |
| mana | TS/Bun | rule classification (+ magic keys) | no | no | e200, mr, example |
| cyanophage | JS | metrics + editable effort matrix | yes (optimiser weights) | yes (web optimiser) | word lists, 15 languages |
| keysolve-web | JS | viewer + quartile percentiles | no | no | pre-baked ngram JSONs |
| layouts-playground | JS/static | trigram-pattern display | no | no | baked |
| keyboard-layout-battles | Rust+Svelte | human typing battles / ELO | human rating | n/a | n/a |
| **ours** | **Python** | **trigram-DP cost + annealer** | **yes (4.3877 objective)** | **yes (anneal/ILS/quench, floor-proven)** | **Norvig big.txt, OpenSubtitles, Gutenberg, Spanish** |

---

## 3. Scoring-model taxonomy

| Family | Members | Unit scored | Notes |
|---|---|---|---|
| A. Weighted metric soup | oxeylyzer, keycraft, a200 (sort weights), cyanophage, Keyrita, genkey (hybrid) | % frequencies → weighted sum | weights are hand-tuned consensus; each tool's composite is not comparable to another's |
| B. Effort/finger-speed | genkey (KPS), cyanophage (effort matrix), (carpalx lineage) | per-finger speed / position effort | physically flavored; SFB enters as speed penalty |
| C. Classification, no composite | cmini, mana, keysolve-web, layouts-playground | counts / triple classes | report-only; ranking left to user |
| D. Exhaustive bigram prefilter | algae | bigram weight only | prunes search space; no trigrams |
| E. Human preference | keyboard-layout-battles | ELO / comfort votes | not a metric tool |
| **F. Ours: trigram-DP cost** | **scorer.py** | **min-cost trigram typing plan + balance** | **fingering chosen per trigram from alt sets (premiums), not fixed; only tool in F** |

The family split explains most disagreements below: tools in A/B *cannot see* what F does.

---

## 4. Tool-by-tool notes (source-verified)

### genkey (Go, semilin)
- Weights: `config.toml:40-41` SFB=1.0, DSFB=0.5; `:58` IndexBalance=0.3;
  `:59` Lsb=1.0; `:61-63` `[Weights.Score.Trigrams] Enabled=false` — **trigrams off by
  default**; `:98` max skipgram size configurable; stagger flag switches X distance
  (`layout.go:586-591`).
- Score ≈ Fspeed×3 + IndexBalance×0.3 + Lsb×1 (`generate.go:45-46` shows the LSB term;
  `layout.go:293` normalizes finger speeds by `l.Total`).
- **Denominator quirk (important):** headline `SFBs` = same-finger bigram count ÷
  layout monogram sum (`layout.go:187` + `output.go:106`), while per-pair shares use
  non-space bigrams (`output.go:141`). Explains our 3.624% vs keycraft's 4.48% on the
  same corpus and same numerator: 4.48/3.624 = 1.236 = bigram/monogram ratio exactly.
- LSB definition: adjacent-finger pairs (LP-LR, LM-LI, RI-RM, RR-RP) with column
  distance ≥ 2, both directions (`layout.go:581-640`).
- Skipgrams with geometric decay `powers[length-i-2]` (`text.go:126`).
- Layout files: 7 lines (name / 3 key rows / 3 finger rows), registered by lowercase
  line-0 name; per-row finger matrices supported (default files use standard map).

### oxeylyzer (Rust, O-X-E-Y)
- **Skipgram semi-decay** — the elegant one: `oxeylyzer-core/src/analyzer_data.rs:142`
  `dsfb_ratio = weights.sfs / weights.sfbs`, applied `ratio^k` for skip distances
  k=1..3 (`:151-153`), all i64 integer arithmetic.
- `trigram_patterns.rs:7-25`: 14-pattern enum (incl. `BadRedirect :23`,
  `BadRedirectSfs :25` = pinky/ring redirections); diagnostics, not the composite.
- Models `SHIFT_CHAR` (⇑) and `REPEAT_KEY` (↻) in ngram data.
- `shai.json` (~2 MB) precomputed monograms/bigrams/skipgrams(1-3)/trigrams.
- Prior run on our layout (standard fingering, shai): SFB 4.105%, score 0.100,
  redirects 4.59%, rolls 35.1%.

### cmini (Python, Apsu — Discord bot)
- `util/analyzer.py:6` loads `table.json` = 10³ finger-triple → class lookup: fast
  trigram classification without full ngram structures.
- `cmds/rank.py:28-44` `STATS`: alt, sfb, sfs, red, oneh, inroll, outroll, roll,
  ratios — **user picks the sort metric; no baked composite**.
- 18 corpora under `corpora/` (the largest corpus menu of any tool), per-layout JSON cache,
  rank/filter with % weights (`rank.py:95-100`).
- No effort matrix, no scissors, no percentile report.

### 200-analyzer (Python, Apsu — `a200`)
- `src/analyzer.py` `get_table()`: 10³ triples → alternate / roll-out / roll-in /
  oneh-in / oneh-out / redirect / sfb / dsfb-red / dsfb-alt / sfT.
- Its "SFB" is a **trigram-class share** (`src/main.py` count_trigrams →
  `trigram_type='sfb'`), not a bigram % — different quantity than genkey/keycraft.
- Thumb space LT/RT/NONE (`parse_args` 'space'); NONE drops space-bearing trigrams
  (we measured 55% "unknown" in that mode).
- Custom sort weights: `sort 70%roll -30%redirect` syntax; percentile-colored output
  (`print_color` at `main.py:35` colors each value by fraction of layouts it beats).
- Bug hit during this study: single-mode never matches dashed layout names
  (`main.py:576-579` adds `cfge` while the file registers `cfge-semiq`) — rename the
  name line to a single lowercase token to work around.

### Keyrita (C#/WPF + native C++)
- `Measurements/LayoutScore.cs:20-25`: KEY_LAG 1.0, ROLES 10, REDIRECTS 25,
  **BAD_REDIRECTS 80**, ONE_HANDS 2, ALTERNATIONS 5.
- Measurements graph: Alts, FingerBalance, FingerLag, HomeUsage, LayoutScore,
  OneHands, Redirects, Rolls, SFN, Scissors; `ScissorMapSetting.cs` makes scissor
  pairs configurable.
- Corpus = user-supplied text (`LanguageSettings.cs`); KLC import; incremental
  recompute on swap events (interactive editor, not batch search).

### keycraft (Go, rbscholtus) — the richest off-the-shelf analyzer
- Default weights `data/config/weights.txt`: SFB −8.0, LSB −4.0, FSB −2.666,
  HSB −1.333, SFS −3.0, LSS −1.0, FSS −0.666, HSS −0.333, RED-WEAK −2.666,
  RED-NML −1.333, **FLW +8.0**, HLD −0.5, FLD −1.5, RLD −1.0, POH −2.0.
- Normalization: median + IQR across the reference set (`scorer.go`), so scores are
  relative, not absolute.
- Metric definitions in `README.md:214-248`: FSB = Full Scissor Bigram,
  HSB = Half Scissor Bigram, SFS/LSS/FSS/HSS = same-finger/lateral/full/half
  scissors on skipgrams, 2RL/3RL trigram rolls, FLW = flowiness, plus
  HLD/FLD/RLD load deviations vs targets and POH pinky-off-home.
- Search: Breakout Local Search (BLS) optimizer + generator, pins, SplitLayout.
- `.klf` format (`internal/keycraft/layout.go:264`): rowstag/anglemod/ortho/colstag,
  12 keys/row (6+6), 6 thumb keys; `~` empty, `_` space.
- Ships **`data/corpus/shai.txt.json` (Shai's Cleaned iWeb, 90M words)** — the
  community-standard corpus we lacked (see F2).

### algae (Rust, ExternalTime)
- `generation.rs:33` `Weights([[u64;30];30]])` — bigram frequencies only; branch-and-
  bound column-wise enumeration with symmetry reduction and SFB-cutoff pruning.
- A search prefilter, not an analyzer: no trigrams, no composite, no fingering alts.

### mana (TypeScript/Bun, Zakkkk)
- `src/analyse/rules.ts` exports the full rule set: finger freq, heatmap, hand
  balance, SFB, LSB, half/full scissors + skip variants, SFR, SFS, LSS, SFSR,
  alternates, redirects, redirect-weaks, in/out rolls, in/out 3-rolls.
- **Unique: magic-key rules** (repeat handling such as `d* -> do`) — no other tool
  except cyanophage's magic page and oxey's ↻ models repeats.
- No composite score, no spacegrams; corpora e200/mr/example.

### cyanophage (JS web)
- `optimiser.js:24-34` composite: SFB w4 (min 0.3), Effort w4 (**user-editable
  per-key effort matrix**, `rcdata` rows), pSFB 0.7, rSFB 0.3, Scissors 0.8 (min 0.2),
  PR-Scissors 0.8 (min 0.2), Wide-Scissors 0.5, LSB 0.5 (min 0.12), SFS 0.2,
  Vowels-same-side 3, Hand balance 0.7 (min 1).
- `worker.js` does the number crunching; word-list corpora for 15 languages;
  `magic.html` covers magic keys.

### keysolve-web (JS web, va1orance)
- **`percentiles.json`: quartile-style boundaries (4 values/metric) for 40+ metrics** —
  the only machine-readable community percentile table we found. Examples:
  - SFB [0.80%, 1.01%, 1.23%, 2.09%]
  - LSB [1.33%, 1.92%, 2.40%, 3.38%]
  - SFS [5.94%, 6.46%, 7.29%, 8.06%]
  - ALT [27.5%, 32.0%, 35.5%, 38.5%]
  - RED [2.36%, 3.09%, 4.07%, 5.37%]
  - Hand: LH [42.7–49.5%], RH [46.7–53.5%]; per-finger loads LI/RI/RM/LR/...
  - plus quad ratios (CAQ/SAQ/CRQ/BTQ/TRQ/RDQ) and row/roll/onehand quartiles.
- `js/classify.mjs`: STANDARD and ANGLE finger maps, thumb toggle; baked corpora
  (mono/bi/skip/tri/tetra grams); viewer only — no optimizer.

### layouts-playground (O-X-E-Y)
- `playground/js/trigram_patterns.mjs:39` `is_bad_redir` + pattern classification
  display; `where_the_magic_happens.mjs`; layout showcase — **no scoring at all**.

### keyboard-layout-battles (malfple)
- Typing-battle platform: human A/B ELO ratings and comfort votes replace metrics.
  Different purpose; kept in inventory for completeness only.

### Ours (analysis/)
- `scorer.py:27-39` weights; `build_tables :111-161` trigram DP over slot triples;
  balance penalty ×1.5 + hand ×0.5 (`search.py:58-91` `Evaluator.score`).
- `fingers.py`: alt sets per position (`:40-117`), premiums `STRENGTH :128`,
  `IDEAL_SHARE {P .137, R .226, M .316, I .321} :135`, travel :154, key effort :178,
  scissor cost tiers :232; arched-home defaults per README rule A.2.
- Corpus: Norvig big.txt (5.08M letters) + held-out OpenSubtitles 50k + Gutenberg +
  Spanish (6.7M); dead keys and ISO-102 apostrophe modeled.
- Search: SA anneal → exhaustive 1-swap / 2-swap / 3-cycle quench → 30 ILS kicks →
  17 long restarts (~5.1M evals) — floor proven, winner rediscovered twice.

---

## 5. Capability matrix (✓ yes · △ partial · ✗ no)

| Capability | Ours | oxey | genkey | keycraft | cmini | mana | cyanophage | keysolve | a200 | Keyrita | algae | klb |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Context-aware alt fingering (DP per trigram) | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Trigrams in the score | ✓ | △ (patterns only) | ✗ (off by default) | ✓ (classes) | △ (triple lookup) | △ (report) | ✗ | △ (baked data) | ✓ (classes) | △ (KeyLag) | ✗ | ✗ |
| Skipgram decay (ratio^k) | △ (SFS tiers) | ✓ | ✓ (geometric) | ✗ (raw) | △ | △ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Scissors (fine-grained) | ✓ (tiers) | ✗ | ✗ | ✓ (FSB/HSB + skips) | ✗ | ✓ (+skips) | ✓ (3 flavors) | ✓ (FSB/HSB) | ✗ | ✓ (configurable) | ✗ | ✗ |
| Explicit lateral-stretch (LSB) metric | ✗ | △ | ✓ (dist≥2) | ✓ | ✗ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| Effort / travel model | ✓ (spec) | ✗ | ✓ (travel+stagger) | ✗ | ✗ | △ (heatmap) | ✓ (editable matrix) | △ | ✗ | △ (KeyLag) | ✗ | ✗ |
| Hand balance inside objective | ✓ (×0.5) | ✗ | △ (index only) | ✓ (HLD) | ✗ | △ (report) | ✓ | △ (quartiles) | △ | ✓ | ✗ | ✗ |
| Finger/row target loads | ✓ (finger shares) | ✗ | △ (index bal) | ✓ (FLD/RLD/POH) | ✗ | ✗ | ✗ | △ (quartiles) | △ | ✓ | ✗ | ✗ |
| Percentile placement of results | ✗ | ✗ | ✗ | ✓ (rank) | △ (color ranks) | ✗ | ✗ | ✓ (40+ metrics) | ✓ (color) | ✗ | ✗ | ✗ |
| Magic / repeat-key modeling | ✗ | ✓ (↻) | △ (skip data) | ✗ | ✗ | ✓ (rules) | ✓ (magic page) | ✗ | ✗ | ✗ | ✗ | ✗ |
| Shift-layer modeling | ✗ | ✓ (⇑) | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Dead keys / punctuation placement | ✓ | ✗ | △ | △ | △ | ✗ | △ | ✗ | △ | △ | ✗ | ✗ |
| Corpus breadth | △ (4) | △ (1 big) | △ (2) | △ (4) | ✓ (18) | △ (3) | ✓ (15 word lists) | △ | △ | △ (user) | △ | ✗ |
| Search / optimizer | ✓ (anneal+quench+ILS) | ✗ | ✓ (generate) | ✓ (BLS) | ✗ | ✗ | ✓ (web) | ✗ | ✗ | △ (interactive) | ✓ (exhaustive, bigram) | ✗ |
| Held-out corpus validation | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Human-preference signal | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ (ELO) |

Only column 1 of this table is unique to us; only keysolve/a200/cmini beat us on
percentile-style reporting; only mana/oxey/cyanophage model repeats.

---

## 6. Cross-validation of cfgE_semiQ (all measured, this + prior session)

Layout under test: `;aiojvlnrw / hfpeygtcms / bxquzkd,./` (space right thumb,
apostrophe ISO-102, dead keys `' . ; /`).

### 6.1 SFB agreement across tools

| Tool | Corpus | Convention | SFB | Top pairs (share) |
|---|---|---|---|---|
| ours, fixed standard fingering | Norvig | bigram | 4.146% | ou 1.06, nc 0.41, ld 0.28 |
| ours, arched default fingering | Norvig | bigram | 3.531% | io 0.64, nc 0.41, po 0.30 |
| **ours, DP alt-fingering** | **Norvig** | **bigram** | **0.020%** | (resolved by alts) |
| genkey | shai-iweb | bigrams ÷ monograms | 3.624% | ou 1.177, yo 0.446, nc 0.329, ld 0.248 |
| keycraft | shai.txt | bigrams incl. punct | 4.48% | ou 1.16, yo 0.44, nc 0.33, ld 0.25 |
| oxeylyzer | shai.json | integer score input | 4.105% | — |
| a200 | monkeytype-quotes | trigram-class share | 4.02% | — |

Reading:
- Pair-level agreement is excellent (genkey vs keycraft: ou 1.177 vs 1.16 etc.), i.e.
  everyone agrees on *which* pairs hurt under standard fingering.
- Total-level differences (3.62 vs 4.48) are pure denominator convention: same
  numerator, monogram vs bigram totals (ratio 1.236 measured, matches exactly).
- Community reference points on the same footing (genkey): colemak 1.09,
  workman 2.37, qwerty 5.25. Our fixed-fingerprint 3.6–4.5% sits well above every
  modern layout — **because the design deliberately converts default-finger SFBs into
  alternation via alts** (the ou/io example: standard map makes `ou` a 1.18% same-
  index pair; our arched default types `o` with the middle finger, and the DP never
  lets `ou/io/nc` land on one finger — 0.02% residual).

### 6.2 genkey full table (shai-iweb, standard fingering, all measured)

| Layout | SFB% | DSFB% | LSB% | Rolls l+r% | Alt% | Redirect% | Onehand% | Score (↓) |
|---|---|---|---|---|---|---|---|---|
| semimak | 0.703 | 5.799 | 1.98 | 42.81 | 40.08 | 6.75 | 2.04 | 32.16 |
| gallium | 0.757 | 5.807 | 1.04 | 44.52 | 42.27 | 3.04 | 1.72 | 30.99 |
| sturdy | 1.024 | 5.960 | 1.65 | 47.39 | 36.01 | 5.52 | 1.83 | 33.45 |
| colemak | 1.092 | 7.903 | 2.84 | 46.68 | 30.81 | 10.74 | 2.51 | 46.11 |
| workman | 2.370 | 7.500 | 1.40 | 44.23 | 30.00 | 10.53 | 3.14 | 54.34 |
| **cfge_semiQ (ours)** | **3.624** | **8.661** | **1.42** | **40.41** | **39.98** | **4.00** | **0.39** | **82.17** |
| qwerty | 5.250 | 9.642 | 5.51 | 37.84 | 26.81 | 13.36 | 2.83 | 115.58 |

Notable: under standard fingering we are mid-pack on LSB (1.42), 2nd on alternation
(39.98), 2nd on redirects (4.00), **best on onehands (0.39)** — but SFB/DSFB sink the
composite (82.17, only QWERTY worse). Genkey's Score cannot see the alt-fingering.

### 6.3 keycraft rank on shai corpus (n=151 layouts, default weights)

Excerpt (score = median/IQR-normalized weighted sum, higher better):

```
 23 graphite      +11.65   SFB 1.02  LSB 0.83  RED-NML 2.03  FLW 81.98
 39 gallium-v2     +7.17   SFB 0.91  LSB 1.49  RED-NML 2.02  FLW 81.50
 53 sturdy         +4.19   SFB 0.90  LSB 1.25  RED-NML 3.28  FLW 80.09
142 colemak       -59.04   SFB 1.35  LSB 3.16  RED-NML 6.22  FLW 72.58
150 cfge (ours)  -156.08   SFB 4.48  LSB 0.37  RED-NML 2.58  FLW 70.34
151 qwerty       -298.27   SFB 6.52  LSB 3.36  RED-NML 7.38  FLW 60.21
```

Percentile positions of our layout (how many of 151 layouts beat it):

| Metric (lower better unless noted) | Ours | # layouts better | Placement |
|---|---|---|---|
| LSB | 0.37% | 8 | **top 13 (5%)** |
| HSS (half-scissor skipgram) | 3.79% | 12 | **top 13 (8%)** |
| RED-NML | 2.58% | 33 | top 34 (22%) |
| LSS | 1.08% | 46 | 47th (31%) |
| RED-WEAK | 0.55% | 79 | mid (52%) |
| HSB | 3.86% | 101 | 102nd (67%) |
| FLW (higher better) | 70.34% | 146 | 147th |
| SFS | 9.35% | 147 | 148th |
| FSB (full scissors) | 0.79% | 148 | 149th |
| SFB | 4.48% | 149 | **150th** |

Also measured: hand balance HLD 0.86 (near-perfect), ALT 38.60% (at/above keysolve's
top ALT quartile bound 38.5%), rows 43.0/45.6/11.4, RLD 58.76, FLD 25.00, POH 5.05.

### 6.4 What the community numbers mean for us — three causes

1. **Fingering model mismatch (dominant).** Every tool fixes one finger per key
   (standard or angle). Our objective picks fingers per trigram from alt sets with
   premiums (`fingers.py:40-117`); default arched map ≠ standard map. Under their
   assumption we are a bottom-5% SFB layout; under ours the residual is 0.02%.
   No tool in the landscape can score the layout as designed (F1).
2. **Denominator conventions** (space/punctuation in numerator and denominator,
   monogram vs bigram totals) shift absolute SFB by ~±1pp even between two tools on
   the same corpus (F3).
3. **Our accepted costs.** Bottom-row punctuation (FSB scissors), split row usage
   (RLD), and an alternation-heavy profile are priced low by our weights
   (trigram cost + balance) and heavily by theirs (SFB ×8, FLW ×8). Same layout,
   different value systems — our annealer proved no layout beats 4.3877 under *our*
   value system; keycraft's top-20 layouts would lose to us on *our* axis too
   (graphite 5.628, gallium 5.660 in `results/final_comparison.json`).

---

## 7. Findings

**F1 — The central incomparability.** No community tool models trigram-context
alternate fingering; ours is the only scorer where finger choice is a decision
variable (`scorer.py` DP). Cross-tool SFB comparisons are only valid under matched
fixed fingering, and then we look bad (3.6–4.5%) by construction. Any external
number must be published with its fingering + denominator convention attached.

**F2 — Shai iWeb is the lingua franca corpus** (genkey, oxeylyzer, cmini, keycraft
all ship it; layouts.wiki uses Reddit). Now materialized in-repo at `corpus/shai/`
(converted from genkey's JSON by `analysis/shai_compare.py`; source PDF:
`corpus/extras/iweb-corpus-samples-cleaned.txt` — 526MB, the exact generator of
shai-iweb.json, verified: its 341,073,092 bigram pairs match genkey's stated total).

**F3 — Even paired tools disagree on totals** (genkey 3.624 vs keycraft 4.48 on the
same corpus/pairs). Compare pair lists or rank positions, never raw headline
percentages across tools. **Denominator mechanism now fully resolved** (A2):
genkey divides the same-finger pair sum by Σ monogram counts *of the layout's own
30 on-layout characters* (~428M, no space) — python recheck 3.627% vs genkey's
3.624%; keycraft divides by all bigram pairs including punct (raw recheck 4.550%
vs its 4.48 header — small header/count offset); keysolve divides by all gram
counts of its own corpora. Same numerator family, three different denominators.

**F4 — Design-risk statement (for LAYOUT.md consumers):** the 0.02% figure is
realized only when `ou, io, nc, po, fa, ld…` are actually typed with the specified
alt fingers. A typist imposing standard habits gets 3.6–4.5% SFB — QWERTY territory.
The layout *requires* the arched/alt discipline; community tables will otherwise
"confirm" it is terrible.

**F5 — Percentile reporting**; keysolve's `percentiles.json` (quartiles for
40+ metrics incl. hand/finger loads) and a200's "fraction of layouts beaten" coloring
are the two reusable patterns. **Now adopted** (A3): `analysis/keysolve_suite.py`
replays keysolve's exact definitions over its corpora and reports our band
placement per metric (`results/keysolve_report.{txt,json}`).

**F6 — Search-methodology uniqueness stands.** Nobody does our quench + ILS +
restart floor proof (algae is exhaustive but bigram-only; keycraft BLS is the
strongest third-party optimizer but has no certificate-of-optimality concept).

**F7 — Where we already look good externally:** LSB (top 5% in keycraft, mid-pack
in genkey), alternation (2nd in genkey; band 2/5 vs keysolve's universe),
redirects (2nd in genkey), onehands (best in genkey), hand balance (HLD 0.86). When presenting the
layout outside this repo: lead with alt-aware SFB + alternation/redirect/onehand
stats; disclose the standard-finger SFB and the fingering requirement (F4).

**F8 — Reusable assets found in the clones:** shai corpora (F2), keysolve
percentiles (F5), keycraft's `load_targets.txt` row/finger targets, oxeylyzer's
skipgram decay formula, genkey's LSB geometry, cmini's `table.json` triple lookup
(speed).

---

## 8. Adoption recommendations (prioritized)

| # | Adopt | Source | Effort | Why |
|---|---|---|---|---|
| P1 | Score + report on shai-iweb alongside Norvig | genkey/keycraft copies (F2) | low | makes our numbers directly comparable to the whole landscape |
| P2 | Percentile placement column in reports | keysolve `percentiles.json`; a200 `print_color` logic | low | one number beats a raw %; quartile context for every metric |
| P3 | Explicit LSB metric | genkey `layout.go:581` (adj-finger dist≥2) or keycraft def | low | our only conspicuous metric gap; community weighs it heavily |
| P4 | Skipgram semi-decay `ratio^k` for SFS ladder | oxeylyzer `analyzer_data.rs:142-153` | medium | smoother than binary SFS tiers; genkey uses geometric too |
| P5 | Repeat/magic-key modeling | mana `rules.ts`, oxey ↻ | medium | repeats ≈ 2.8% of bigrams (AKL research) — biggest non-swap win |
| P6 | Row/finger load deviations vs published targets | keycraft `load_targets.txt`, keysolve finger quartiles | low | our RLD/FLD blind spot (we only balance fingers/hands internally) |
| P7 | Shift-layer ngram model | oxeylyzer ⇑ | future | only relevant if we ever optimize the shift layer |
| P8 | Editable effort-matrix sensitivity harness | cyanophage `rcdata`/`optimiser.js` | optional | complements existing `letter_sensitivity.txt` |

Skip: keyboard-layout-battles (human ELO, different purpose), algae (bigram-only
prefilter — our quench already dominates), cmini (bot infra, but keep its corpus
list as a source list), Keyrita (Windows UI; its weight table is already transcribed
above).

---

## 9. Reproduce

```bash
# clone (dest names match §1)
D=/tmp/opencode/analyzers
mkdir -p $D
git clone --depth 1 https://github.com/semilin/genkey            $D/genkey
git clone --depth 1 https://github.com/O-X-E-Y/oxeylyzer         /tmp/opencode/oxeylyzer
git clone --depth 1 https://github.com/Apsu/cmini                $D/cmini
git clone --depth 1 https://github.com/Apsu/200-analyzer         $D/200-analyzer
git clone --depth 1 https://github.com/youngbrycecode/Keyrita    $D/Keyrita
git clone --depth 1 https://github.com/rbscholtus/keycraft       $D/keycraft
git clone --depth 1 https://github.com/ExternalTime/algae        $D/algae
git clone --depth 1 https://github.com/Zakkkk/mana               $D/mana
git clone --depth 1 https://github.com/cyanophage/cyanophage.github.io $D/cyanophage
git clone --depth 1 https://github.com/va1orance/keysolve-web    $D/keysolve-web
git clone --depth 1 https://github.com/O-X-E-Y/layouts           $D/layouts-playground
git clone --depth 1 https://github.com/malfple/keyboard-layout-battles $D/keyboard-layout-battles

# genkey (build; needs a run dir with config.toml + layouts/ + corpora symlink)
cd /tmp/opencode/analyzers/genkey && go build -o /tmp/opencode/genkey/genkey .
mkdir -p /tmp/opencode/genkey-run/layouts
ln -sfn /tmp/opencode/analyzers/genkey/corpora /tmp/opencode/genkey-run/corpora
# layout file: line0 = registered lowercase name; 3 key rows (space-separated); 3 finger rows
cd /tmp/opencode/genkey-run && /tmp/opencode/genkey/genkey analyze cfge_semiq

# keycraft (layout file data/layouts/cfge.klf, run from repo root)
cd /tmp/opencode/analyzers/keycraft && go build -o /tmp/opencode/keycraft ./cmd/keycraft
/tmp/opencode/keycraft analyse cfge -c shai.txt
/tmp/opencode/keycraft rank -c shai.txt          # n=151 table

# 200-analyzer (name line must be a single lowercase token — see §4 bug)
cd /tmp/opencode/analyzers/200-analyzer && ./a200 space rt && ./a200 view cfge

# keysolve percentiles
python3 -c "import json;print(json.load(open('/tmp/opencode/analyzers/keysolve-web/percentiles.json'))['SFB'])"
```

Raw outputs from this study: `/tmp/opencode/keycraft_rank_all.txt`,
`/tmp/opencode/keycraft_cfge_shai.txt`, `/tmp/opencode/a200_cfge*.txt`.

---

## 10. Follow-up experiments (A1–A4, B6, B8, D1–D6)

Second session, user-selected follow-ups. All measured, all reproducible via
the scripts in `analysis/`; raw outputs in `results/`.

### 10.1 A1 — Cross-corpus check on shai-iweb (the lingua franca, F2)

`analysis/shai_compare.py` scores the 12-layout field on `corpus/shai/`
(genkey corpus, 418,381,828 letter / 15,437 trigram tokens):

| # | layout | shai score | Norvig score |
|---|---|---|---|
| 1 | **cfgE** | **4.4404** | 4.3877 |
| 2 | cfgG | 4.4751 | 4.4010 |
| 3 | cfgD | 4.7039 | 4.6034 |
| 4 | cfgF | 4.7039 | 4.6034 |
| 5 | Graphite | 5.6890 | 5.6280 |
| 6 | Sturdy | 5.6937 | 5.5760 |

Top-4 identical to the Norvig ordering; cfgE's margin over the best community
layout (Graphite 5.69) is 1.25 shai-points vs 1.24 on Norvig — the win is not
an artifact of one corpus. 6/12 rank positions match exactly; the rest shuffle
within noise. (`results/shai_comparison.{txt,json}`)

### 10.2 A2+A3 — keysolve-compatible metrics, percentile bands, SFS ladder

`analysis/keysolve_suite.py` ports keysolve-web's `classify.mjs`/`stats.mjs`
definitions (standard finger map, any-distance skipgrams, ortho board,
denominator = all gram counts) over keysolve's own corpora; bands from its
`percentiles.json` (5 bands, band 1 = best). Our placement:

| metric | value | band | reading |
|---|---|---|---|
| SFB | 5.76% | 5 (worst) | standard-finger view only — blind to our alts (F1/F4); keycraft-style alt-aware numbers live elsewhere |
| LSB | 1.99% | 3 | mid |
| HSB | 4.17% | 3 | mid |
| FSB | 0.53% | 4 | mildly high |
| SFS | 9.96% | 5 | high — see ladder below |
| LSS / HSS | — | 1 / 1 | best band (scissor design vindicated on keysolve's own metric) |
| FSS | — | 5 | same-finger same-row family rides on standard-finger SFB |
| ALT | 36.28% | 2 | above-median alternation (top quartile needs ≥38.53) |
| ROL | 39.36% | 5 | below keysolve's Q1 bound → rolls rarer than most layouts in its universe — alternation-heavy profile under this definition (our internal trigram taxonomy's 41.4% roll share uses different conventions; do not cross-read) |
| ONE | — | 1 | best: fewest one-hand runs |
| RED | — | 3 | redirects mid |

**SFS distance ladder** (`analysis/sfs_ladder.py`, raw iweb, standard
fingering, repeats excluded): SFS₁ 9.477%, SFS₂ 9.294%, SFS₃ 9.068%,
SFS₄+ 9.179%, all-gap 9.282%, oxey-style decay ladder (r=1/7) 9.446% —
essentially flat across distance; no decay story to tell, so we report the
plain tiers. SFB denominator cross-checks: bigram-denominator 4.550%
≈ keycraft's 4.48 header (consistent), monogram-on-layout 3.627%
≈ genkey's 3.624% (consistent) — closing F3. (`results/keysolve_report.*`,
`results/sfs_ladder.txt`)

### 10.3 A4 — Robustness of the win to weight choices

`analysis/robustness.py` perturbs the model in 18 scenarios (roll/alt/
redirect/scissor groups ×0.5/×1.5/×3, balance ×0/×1.5/×3, hand ×0/×2,
pessimistic joint) by rebinding the weight globals in `scorer.py`,
`search.py`, `fingers.py` (edits in this session made all knobs module-level;
baseline re-asserted 4.3877 after edits):

- **cfgE is #1 in 17/18 scenarios** under quench.
- Sole static flip: balance ×3 → cfgG 4.4223 vs cfgE 4.4263 (margin 0.004);
  quench from either incumbent then reaches 4.4067 (a sibling re-shuffle,
  `…cmsbkxuzqd,./`) — the family is stable, only the label moves.
- Quench finds improvements only under extremized groups (worst −0.0235);
  layouts change only then — otherwise the winner is a fixed point.
- Anneal-8k restarts never beat quench from the incumbent.
  (`results/robustness.{txt,json}`)

### 10.4 B6 — Punctuation priced into the objective

Motivation: the letters-only objective never sees punct mass (2.29% of
keystrokes), and cfgE parks `.` and `/` **both on the right pinky** on top of
`s` (Kuntem's criticism); `s.` is the 3rd-biggest punct bigram (0.896M pairs)
and a same-finger pair under cfgE.

- `analysis/build_punct_corpus.py` → `corpus/punct/` (Sigma = a-z + `,. /;`,
  apostrophe breaks runs, `?`→`/`, `:`→`;`; 428.2M chars).
- `analysis/search_punct.py`: 30-char search reusing the 31-key tables, score
  = trigram + balance + hand with punct mass included.

| variant | punct-priced | letter-only |
|---|---|---|
| cfgE as-is | 4.5526 | 4.3877 |
| letters re-optimized, punct frozen | 4.5387 | 4.4125 |
| **joint re-search (winner)** | **4.4652** | 4.4573 |
| naive off-pinky swaps, no re-search | 4.9474 / 5.1699 | — |

Findings: (1) naive punct swaps cost +0.39–0.62 — the fix requires joint
re-search; (2) letters barely move (−0.014 of the −0.087 gain) — cfgE's
letter placement is robust to punct pricing; (3) the joint optimum
`.aiozvlnrbhgpeymtcwsfjxu/kd,;q` was reached from **both** seeds (stable),
relocating `.`→Q (kills the `s.` pinky pair by cross-handing it), `;`→pinky
bottom, `/`→left index, `,` stays; (4) cost: **+0.070 letter-only**.
Verdict: primary cfgE unchanged; optional `cfgE-punct` variant documented in
`results/LAYOUT.md`. (`results/b6_punct.{txt,json}`, `results/b6_ablation.json`)

### 10.5 B8 — Hybrid EN+ES side layouts (English primary, no compromise)

`analysis/hybrid_search.py`: blend = (1−w)·EN + w·ES on normalized L/B/T;
per blend, cfgE-warm + 5 random anneal/quench restarts; every result scored
on **both pure axes**. w=0 control reproduces cfgE exactly.

| w | EN | ΔEN | ES | ΔES | note |
|---|---|---|---|---|---|
| 0.00 | 4.388 | — | 5.483 | — | cfgE (installed) |
| 0.20 | 4.457 | +0.070 | 5.644 | +0.161 | dominated |
| **0.35** | **4.503** | **+0.116** | **4.535** | **−0.948** | **knee** |
| 0.50 | 4.775 | +0.387 | 4.722 | −0.761 | dominated |
| 0.65 | 4.858 | +0.470 | 4.584 | −0.898 | dominated |
| 0.80 | 4.911 | +0.523 | 4.357 | −1.125 | ES-heavy |

- Knee `hybrid-35` = `;aemjwcioksfgnhptubrqzylxvd,./`: **Pareto-dominates every
  community reference on both axes** (best ref EN = Sturdy 5.576, best ref ES
  = Sturdy 6.377); −17% Spanish score for +0.12 English.
- w=0.8 also beats all refs both axes.
- Math note: the trigram term blends linearly (verified exact), but
  balance/hand penalties are quadratic in the blended usage — blend score ≠
  convex combo of pure scores, so blend-optima can be pure-axis-dominated
  (w=0.2); those are reported as dominated, not hidden.
- All variants keep the same 30-key alphabet; Spanish ñ/accents/¿¡ are
  outside the modeled alphabet by design.
  (`results/hybrid_pareto.{txt,json}`)

### 10.6 D-series — autonomous improvement campaign (all arms negative)

Question: can cfgE (4.3877) be improved by harder search, a different
alphabet, a different objective, weights, or a third corpus? Full synthesis
in `results/d_series.txt`. Arms:

- **D1 deep search** (`deep_search2.py`): 4-cycle + double-bridge ILS,
  multi-start. warm-ILS held 4.3877 over 50 iters; a **random** start
  converged to exactly cfgE from 4.4486; annealed starts stuck higher.
  Floor re-confirmed under a stronger neighborhood.
- **D2 dead-set** (`deadset_search.py`, corpus `punct2` adds `'`/`-`,
  mass 0.26% each vs `;` 0.10%): 6 configs (`{;, ', -}` × slot `{Q,P}`).
  `;`@Q wins the punct axis again (4.4654); in-grid `'` (4.497) and `-`
  (4.495) lose. Alphabet choice validated; no change.
- **D3 published weights** (`published_weights.py`): ranking the A2 field
  under keycraft `weights.txt`/`weights2.txt` puts cfgE **9/10** both —
  the expected standard-fingering blindness (F1/F4), validation only.
- **D4 load/balance** (`load_search.py`): keycraft `load_targets`
  (hands 50/50, fingers 7/10/16/17, rows 17.5/75/7.5) as extra penalty
  cuts load_dev 6.46→3.74–4.14 but costs +0.37–0.70 original score;
  balance-weight re-search (W∈{0,3,5}) bottoms at 4.3889. **No
  dominance — cfgE is the knee** of the (score, load) frontier.
- **D5 Wikipedia** (`wiki_eval.py`, 200 featured articles, 2.88M letters):
  cfgE **#1** (4.4095); field order ≈ iweb. Third independent English
  source; no threat.
- **D6 space-aware** (`space_eval.py`): space grams are 49.8% of tokens
  (44.2% of cost); cfgE still #1 of the field (3.998); dedicated
  space-aware optimum costs +0.087 English — not adopted.

Conclusion: **cfgE unchanged**; 4.3877 re-confirmed under deeper search,
a third corpus, space-aware scoring, and punct-alphabet variation. The
trade-off surface (D4) and community-weight rankings (D3) are now mapped
in `results/d_*`.

### 10.7 Follow-up reproduce

```bash
cd analysis
python3 shai_compare.py        # A1 → results/shai_comparison.*
python3 keysolve_suite.py      # A2/A3 → results/keysolve_report.*
python3 sfs_ladder.py          # A3 → results/sfs_ladder.txt
python3 robustness.py          # A4 → results/robustness.*
python3 build_punct_corpus.py  # B6 corpus → corpus/punct/
python3 search_punct.py        # B6 → results/b6_punct.*
python3 hybrid_search.py       # B8 → results/hybrid_pareto.*
python3 deep_search2.py 0 50   # D1 → results/deep_search2_s0.txt
python3 build_punct2_corpus.py # D2 corpus → corpus/punct2/
python3 deadset_search.py      # D2 → results/d2_deadset.txt
python3 published_weights.py   # D3 → results/d3_published_weights.txt
python3 load_search.py         # D4 → results/d4_loads.txt
python3 wiki_eval.py           # D5 → results/d5_wiki.txt (+ corpus/wiki)
python3 space_eval.py          # D6 → results/d6_space.txt (+ corpus/word)
```
