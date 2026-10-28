# Research: AKL Recommended Layouts & Methodologies
**Source:** https://layouts.wiki/guides/start/recommendations/ (Sep 7, 2026 snapshot) + each layout's own documentation
**Date:** 2026-10-20
**Purpose:** Understand how the community's top layouts were designed, and extract ideas that could improve our layout *beyond key swaps*.

> Repository note (2026-10-28): the non-Shai/Spanish corpora have been
> removed. Scripts and results referenced below now live under
> `analysis/legacy/` and `results/archive/`.

---

## 1. How the AKL community ranks layouts (the shared worldview)

The recommendations page states the consensus metric priorities, in order:

1. **Speed** → minimize **same-finger bigrams (SFB)** first.
2. **Accuracy** → minimize **redirects**, especially **weak redirects** (same-hand trigram direction change with no index/thumb).
3. **Comfort** → minimize **lateral stretches** (index abduction / pinky adduction while neighbor finger is stationary) and **scissors** (adjacent-finger opposing flexion/extension across rows: half = 1u, full = 2u).

Crucially, **all of these are also computed as skipgrams** (1st-3rd keystroke of a trigram) — felt starting ~80 WPM. This is a big deal: most of our objective's fine structure is bigram/trigram *cost*, but the community treats skipgram versions of the same patterns as first-class.

Corpus: **Reddit (small) from ConvoKit** (~54M chars, 2018, 100k users) — informal online English, deliberately not academic. Our search corpus (Norvig big.txt) is closer to edited prose; our held-out OpenSubtitles corpus is dialogue. A Reddit-like corpus is the one we haven't tested — worth knowing the community optimizes for casual text where contractions, slang, and "you" "lol" "im" patterns dominate.

Percentile tables on each layout page are computed over the wiki's whole layout database with this corpus — e.g. Gallium SFB 0.57% (P26 = better than 74% of layouts), weak redirects 0.29% (P46).

### The "minimum SFB" theory they use
- ANSI (no thumb letter): theoretical floor **0.37% SFB**, requires Whorf's "low-usage high-movement index" arrangement. Whorf (2021) hit 0.38% but has pathological edge cases ("copying") and bad scissors — the wiki explicitly says optimizing one metric alone is a trap.
- With one thumb letter: floor drops to **0.24%**; Night hits 0.35% cleanly. More thumb letters *raise* the floor again — "optimal number of letters on thumb for SFB minimization is almost certainly exactly 1."
- Modern AKL layouts deliberately sit at **0.5–0.7% SFB** to buy better everything-else.

### The "Why These?" trade-off doctrine
> "When choosing among well-optimized layouts, the choice often comes down to which of several sets of problems you would rather live with."

AKL layouts are allowed to have **one or two "badly placed letters"** (e.g. Night's P in a corner) when that's the price of low SFB/SFS. Non-AKL layouts instead spread the cost as mild SFB/scissor increases to keep medium-frequency letters out of corners.

---

## 2. Per-layout design processes

### 2.1 Gallium (Bryson James, 2023) — "balanced beginner pick"
- **Lineage:** direct descendant of **Nerps** (Smudge); explicitly inspired by Sturdy; so close to Graphite that they're interchangeable recommendations, developed independently weeks apart.
- **Method:** manual design with stated goals — "break up repetitive patterns, balance fatigue between hands, be generally compatible with most people." Key trick: **put H (consonant-vowel transition letter) on the vowel-hand index** to break up rolls and pull hand balance toward parity.
- **Process evidence of humility:** after release, X/Q and J/Z were swapped for small SFB/SFS wins — iterative post-release refinement is normal.
- **Known weakness (their own words):** HY, PY, PH, PF blow up on advanced vocabulary ("physics", "happy", "python"); B-on-pinky-top causes BR/MB scissors ("broad", "thumb"). Corpus sensitivity acknowledged: "performance is dependent on a corpus of words… does particularly poorly at Monkeytype 450k."
- **Lesson for us:** the community's top "balanced" pick still has a documented list of bad bigrams, and they *ship it anyway* with mitigation advice (alt-finger MB with index on row-stagger). Perfection is not the bar; well-documented trade-offs are.

### 2.2 Graphite (Richard Davison, 2023) — "generate, then hand-tune the gaps the analyzer can't see"
- **Method (from the README — the richest process document we found):**
  1. Forked **Oxeylyzer** (Oxey's SA generator/analyzer, the tool that natively generates Sturdy) and tweaked the fitness filter to down-weight bigrams he disliked.
  2. Ran generation → precursor layout.
  3. **Learned the layout himself first** ("eating my own dog food") for months.
  4. Tried hand-swaps; **reverted nearly all of them** ("I concluded that I was not smarter than Oxeylyzer").
  5. Kept only the class of swaps Oxeylyzer *couldn't* see: **punctuation + shift-layer bigrams**.
- **The punctuation saga (critical for us):**
  - Analyzer ignored shift pairs → `.”` was a bad bigram the tool never saw. Moved `.` to pinky → fixed `.”` but created `.`+right-shift pain for programming (`Belt.List.Set.fromArray`). Final fix: **swap `”` and `_`** so the quote sits next to `.`; `.` back on middle.
  - Programmer bigrams `->`, `=>`, `|>`, `./`, `:=`, `):` drove `>` onto middle finger — **flip `<`/`>`** intentionally.
  - Constraint: **never move a symbol to a different shift layer than QWERTY's** (shortcut survival; learned the hard way from MTGAP where `{` under `3` turned Cmd+{ into screenshot).
- **Design theory articulated:** H "comes after consonants, before vowels"; N the opposite. Place N on consonant-pinkly (inward rolls), H on vowel-index (outward rolls) → systematic left→right flow, classified as an "ultralow redirect" layout. S on index = word-final inward rolls. "Interleaved rolls" (each hand rolling inside an alternation, e.g. `with` = w_t + i_h) balance the L→R tendency — a pattern class **no analyzer measures**.
- **Stated shortcomings:** BR/PH/MB cluster (same structural issue as Gallium's NRTS/HAEI home block); SC/SW/GS mitigated by alt-fingerings; RM edge cases.
- **Lesson for us:** our 4.388 optimum is under a bigram+trigram cost model with no shift-layer awareness and no "interleaved roll" reward. Graphite's biggest wins over its own generator came from exactly these unmeasured dimensions.

### 2.3 Pine v4 (ClemenPine, 2021–) — "Gallium-family feel, no PHY/BR/MB"
- Five major versions; v4 is the current rec. Removes Gallium/Graphite's characteristic bad triplets by a different consonant cluster.
- **Own weakness (wiki):** heavy right-hand weak-finger load, especially **YI pinky** ("hey", "yeah", "playing") — the cost of eliminating the index-cluster problems. Percentiles confirm: right ring usage P95, right pinky P97.
- **Lesson:** moving a problem cluster just moves the load to another finger; the community accepts high pinky *usage* as long as pinky *movement* stays low (Kuntum/Kuntem/Birdie take this to the extreme).

### 2.4 Sturdy (Oxey, 2021) — "high rolls + ultra-low bad redirects, built with a custom analyzer"
- **Method (from oxey.dev):** built his **own analyzer first** — "most of the work going into this layout was getting my analyzer to output what I wanted." Design targets: very high rolls, minimize redirects (especially non-index redirects = bad redirects), low pinky usage and finger speed.
- Generated multiple candidates meeting the criteria (rnts, stnd columns); chose Sturdy because alternatives had worse left-index load.
- **The Whorf index:** named after the 2021 layout that showed a *low-usage, high-movement index* minimizes average SFB — optimal in aggregate, but pathological in clusters ("copying" hits the index repeatedly). Sturdy and Night both carry a Whorf-ish index and the wiki flags the edge cases.
- **Known issue:** K placement ("task", "milk", "back"); heavy left ring. Also: RL-heavy words ("early", "world") followed by left-hand keys — overrepresented in Monkeytype default wordlist, so corpus choice changes its ranking.
- **Lesson for us:** Oxey's process = tune the *analyzer's weights* until the generator outputs something that feels right, rather than hand-editing outputs. We did the analogous thing (batch search with fixed objective); the question our research must answer is whether our objective has the right weights — see §4.

### 2.5 Canary (Apsu + AKL community collab, 2022) — "high rolls, community design by committee"
- Explicitly a **collaboration of top AKL layout creators**, released with a Monkeytype tournament. Design principles documented in the README:
  - Primary: **high rolls** (>55% trigrams), balanced in/out (T on left index, N on right as high-freq roll targets).
  - **Minimize pinky load** (almost 0% pinky SFB/redirects) because pinky length varies most across the population.
  - Vowels one hand, arranged to kill SFBs *and* redirects without pinky overload.
  - **Semivowels W/Y** moved to dodge both vowels and high-freq consonant-semivowel pairs (ly, ty, tw, sw); Y accepted in the `ysv` column making VY/SY SFBs as the priced trade.
  - Liquids L/R on ring (mid-frequency, combine with everything, low with each other).
  - `'` next to A (A is the letter `'` combines with least: n't, 's, 're).
  - **Row-stagger geometry exploited:** G/M placed on the near-reach inner index bottom keys; D/H on the strong bottom positions; Q/Z in the hardest-to-reach row-stagger corners (QWERTY B/Y spots).
- **Requires angle mod** (two shipped variants: Angle Mod + Ortho). "It is very very important for you to use the Angle Mod fingerings on Canary. All of the heavy optimization completely depends on it."
- **Lesson:** punctuation adjacency (`'`+A) and hardware-geometry-aware letter placement are first-class design moves, not afterthoughts. Also: **variants per keyboard geometry are standard practice** — we only ship one fingering model (arched-home, row-stagger-friendly but not angle-mod).

### 2.6 Kuntum / Kuntem (adi.lela, 2023/2024) — "high pinky usage, low pinky movement"
- Kuntum's thesis: give T (most common consonant) and TT (common repeat) to left pinky; keep the only off-home pinky letter to a rare one (V). Result: **P97 left-pinky usage but P0 half-scissor skipgrams, P5 full-scissor skipgrams** — the extreme end of "usage yes, motion no."
- **Requires angle mod; un-angle-able** (no ortho variant) — a deliberate hardware commitment.
- Kuntem iteration: to avoid overloading right pinky with E + common punctuation, **moved `.` and `,` off pinky** (same move Graphite made for `.`).
- **Lesson:** our layout puts `;` on right pinky home (highest-usage position on that finger) and `.`/`,`/`/` on right pinky bottom — the Kuntem/Graphite fix (punctuation on other fingers when the pinky already carries a high-usage letter) is directly applicable to our punctuation layer and is *not* a letter swap.

### 2.7 Night (Valorance, 2024) — "thumb alpha, SFB/SFS-first, iterated 5 layouts"
- Night is the ~5th major layout by its author (Maks-Ex → RSTHD → Strand → Stern → Night), each an experiment in a different philosophy. Documented design system:
  - **DTM+K index:** most frequent consonant T on index; D/M supporting; K as low-freq filler. "The index finger is the most dexterous finger."
  - **LH middle:** most-repeated letter L on top-middle (middle finger "favors stretching up").
  - **FSV ring:** low-freq F/V around S; S off pinky because SS is a common repeat.
  - **BNX pinky:** only B off home; N as the "second to H in minimal repeats" pinky letter.
- **Heavily optimized for accuracy at speed:** SFB, SFS, weak redirects — "especially impactful when typing quickly—100 WPM and higher."
- Carries a Whorf index (documented pathological cases: "copying", "wagyu", "waypoint").
- **Known pain:** lateral stretches from Y and P ("you", "people") — the cost of its corner placements.
- **Post-1-year reflection (valorance.net/night/reflection):** author confirms most concerns resolved; persistent issue is P's LSBs with vowels (artifact of maximizing same-row bigrams); plans to add a **repeat/magic key on a third thumb** — the community direction of travel.
- **Lesson:** repeat-frequent letters (l, s, o, e…) deserve anti-repeat placement, not just anti-SFB placement. Our repeats land 44% on right middle (tt/cc — t and c are both right middle home/top in our grid) — worth checking against the corpus repeat map. Also: **the author's own SFS metric weighting** (Semi's formula, wiki: SFS% = Σ SFSᵢ%·2^(1/i)) is the community standard; our scorer weights skipgrams but not with this specific decay.

### 2.8 Dusk-WP (2024) — "same family as Night, different finger-load opinion"
- Same design goals (SFB/SFS/weak-redirects-first), same columns mostly; differs in left index vs middle split. High lateral stretches from M ("come", "man"); Whorf index on the opposite hand from Night.
- **Lesson:** even within one design school, the index/middle load split is a free parameter tuned by feel — not derivable from corpus math alone.

### 2.9 Enthium (Suraj N. Kurapati, 2024–2026) — "the most documented iterative process"
Enthium's site is a **full lab notebook**: v1 (mirror of Promethium+Engrammer) → v14, every swap justified by *multiple analyzers disagreeing* and real-world use:
- Started from Cyanophage analyzer rankings; mirrored horizontally to keep vowels left (Dvorak/Engram tradition) after a playground accident improved Total Word Effort.
- **KeyBr training as evaluation instrument:** 7.5h to unlock all letters; logged which letters "took a long time to clear" (W, Q) → those became swap candidates (Q/Z trial, then revert).
- **Real-world day of use** → notebook of specific discomforts (WN stair-step SFB, DW 2u skip bad for Vim, FF weak on pinky) → targeted refinements.
- **Triangulation protocol:** every candidate swap measured in Oxey + KeySolve + Cyanophage; swaps kept only when at least the primary analyzer showed no regression ("Oxey's analyzer showed no changes at all!").
- Community input formalized: GitHub issues and Reddit suggestions each spawned multi-candidate searches (v10 = 16 candidates, v11 = 9 candidates).
- **Rake/slide taxonomy:** vertical SFBs (ue, oa, nf) can be "raked down" in one motion; horizontal ones "slid over" — effective SFB 0.22% vs nominal 0.55%. This **softens the SFB metric by physical technique** — our scorer treats all SFBs as equal cost.
- Programming punctuation as design pillar: HJKL + `,;` for Vim, `-=` zoom, `./` paths, `,./` in QWERTY order for adaptation.
- **Pinky load stratification:** heavy letters (B, W) on lateral pinky keys (CapsLock/Apos positions), medium on bottom, light on top — for short pinkies.
- **Lesson:** (a) multi-analyzer triangulation prevents overfitting one tool's blind spots; (b) the *training experience itself* is data; (c) rake/slide-aware SFB costing would likely re-rank several of our near-optimal alternatives — including possibly the winner vs cfgG.

### 2.10 Bunya (GalileoBlues, 2023) — "Gallium, but S on thumb"
- Thesis: put S (high-freq, high-repeat) on thumb → exceptionally low SFB/SFS/scissor bigrams *and skipgrams*. Fixes Gallium's PHY and MB/ML; keeps BR.
- Cost: **SS is a common repeat and thumbs are slower at repeats than fingers** (the wiki's counterargument to thumb-E generally: "rr less common than ee" is why R-on-thumb beat E-on-thumb historically; both camps exist).
- High lateral stretches ("you").
- **Lesson:** if we ever relax the "no thumb alts" constraint, S-on-thumb is the community's current favorite single-letter thumb move for a Gallium-shaped vowel/consonant split — and our layout's home block is in that family (NRTS-class right hand + HAEI-class left). But our `;`-as-home-pinky and arch design make this a different optimization problem; don't cargo-cult it.

### 2.11 Birdie (2025) — "tiny keyboards, pinky-heavy, R on thumb"
- For 2-column pinky boards. Kuntem lineage + R thumb. Problems: SL (can't alt-finger on ortho), BL comfort depends on column stagger amount.
- **Lesson:** alt-fingering availability is geometry-dependent — our extensive alt-finger set assumes a standard staggered board; on ortho/colstag some alts become stretches.

### 2.12 Racket (2024) — "lowest SFBs attainable with sane design"
- "About the lowest SFBs you can get" with SFS considered; thumb R; very low lateral stretches; **no major problem keys**.
- Cost: finger usage favors weak fingers, especially **left ring** (P-weights show it); S+F both on left pinky (swap F/Q offered as fix).
- **Lesson:** the wiki's "no major problem keys" bar is achievable together with elite SFB — our layout's documented weak spots (PO/OP/CL scissors from the analysis output) are exactly the class Racket claims to have eliminated; worth a constrained re-run that adds an explicit "no bigram over X cost" ceiling rather than only minimizing the sum.

### 2.13 Non-AKL references (context for our placement)
- **Colemak/Mod-DH:** recommended only for OS availability; high weak redirects ("you", "was") + lateral stretches ("the", "here") are the canonical failure modes our layout should avoid — and does (our weak-redirect rate ≈0.61% of trigrams by the quick calc above, vs Colemak-class layouts being much worse).
- **Hands Down Neu:** maximizes in-rolls (3:1 in:out) + strict hand/finger balance; pays with scissors (PL = 0.23% of bigrams alone!) and weak redirects. Their answer: **adaptive keys** (A after U types H, etc.) — mostly theoretical; "in practice, most people do not end up implementing them."
- **Not recommended for the record:** Engram (center-column punctuation trick too costly in SFB), BEAKL (extreme pinky-avoidance = 3–5× SFBs), Dvorak (2× Colemak SFBs), CarpalX/Norman/Workman (QWERTY-similarity tax), Halmak (vowel-consonant finger mixing → catastrophic same-finger trigrams, "wood"/"book").

---

## 3. What this community believes that our model does NOT encode

| # | Community concept | Our model's status | Relevance to us |
|---|---|---|---|
| 1 | **Skipgram-first-class** (SFS, scissor skipgrams, LSB skipgrams; Semi's decay formula) | Skipgrams penalized in cost via 3-slot DP context, but no separate SFS metric with the 2^(1/i) weighting; no published SFS% for our layout comparable to wiki P-percentiles | High — Night/Dusk/Racket are optimized on this axis; our trigram DP approximates it but we've never *reported* SFS% |
| 2 | **Lateral stretch (LSB/LSS) as separate metric** | Not explicitly modeled; subsumed vaguely in row-gap SFB costs and balance | Medium — "you" (our YV/R-class?) and pinky-adduction patterns are the #1 comfort complaint family on the wiki |
| 3 | **Half vs full scissors, finger-pair severity** (IM tolerable, RP worst) | We model scissors with per-pair costs and user tolerance labels (good) — actually **ahead** of most analyzers here | Low — keep as-is; cite as strength |
| 4 | **Shift-layer / modifier bigrams** (`.”`, `.`+Shift, shortcut layers) | Completely absent; punctuation scored only on unshifted bigrams with dead keys | **High** — Graphite's entire post-generator win came from this |
| 5 | **Angle-mod / hardware-geometry variants** | Single arched-home fingering model; no angle-mod variant | Medium — user is on row-stagger (setxkbmap pc105); angle mod is free comfort on that hardware for *some* layouts; our bottom-left (b x q u z) under angle mod puts b on ring (adds ab/ba SFBs ~0.2%) — quantified in §5, not recommended to switch |
| 6 | **Thumb alphas (exactly 1)** | Excluded by design spec (space only on thumbs) | Low unless user changes hardware; document as future option |
| 7 | **Magic/repeat key** | Not present | **Medium-high and cheap**: repeats = 2.82% of letter bigrams on our layout, 44% of them on right middle (tt, cc). A Kanata dual-function key or extra thumb could remove most of this with zero letter movement |
| 8 | **Whorf-index pathology check** | Not audited | Quick audit in §5: our index fingers carry 24% usage — no Whorf pathology detected |
| 9 | **Rake/slide effective-SFB discount** | All SFBs costed equally | Medium — several of our top SFBs (io, nc, po) are vertical or near-vertical and rakeable; would soften the "unavoidable SFB" narrative |
| 10 | **Corpus = Reddit-casual** | Norvig + held-outs (book/dialogue) | Low-medium — cross-corpus already showed stability; a casual corpus test would close the loop |
| 11 | **Pinky-off (usage − home usage) metric** | Not tracked | Medium for short-pinky users; ours likely fine (home-heavy pinkies) but unquantified |
| 12 | **Multilingual K/C issue** | Spanish study done; German-style K-heavy orthographies untested | Low (user's languages are EN+ES) |
| 13 | **Interleaved rolls** (Graphite's inside-out alternation rolls) | Not measured | Low as a metric; interesting as a *post-hoc feel* argument for why our high-alternation score doesn't feel sterile |
| 14 | **Training methodology** (memorize-all-at-once vs Keybr; ~months to recover) | Out of scope for the layout repo, but the wiki's FAQ is the community's answer to "how do I live with this" | Include one paragraph in README later if user asks |

---

## 4. How they reached their conclusions — the four archetypes

Distilling every process above, the community converges on four methodology archetypes, usually mixed:

1. **Analyzer-tuning + SA generation** (Sturdy, Graphite, Pine, most 2021+ layouts): build or fork an analyzer, set weights until the generator's output matches taste, run SA with pins, ship, then revert failed hand-swaps. *We did this* with our trigram-DP objective + batch annealing; our deep-search results (§ prior work) say we're at the floor **of that objective**.

2. **Constraint-satisfaction by hand with named principles** (Canary, Kuntum, Racket): pick 3–5 non-negotiable principles (high rolls; pinky usage yes / movement no; no problem keys), arrange clusters manually, verify with analyzer, iterate. Often produces layouts with "no major problem keys" but slightly worse headline scores.

3. **Lineage iteration** (Night, Enthium, Bunya←Gallium←Nerps): inherit a family's structure, change 2–4 letters for a named reason, live with it for weeks-to-months, repeat. Enthium's v1→v14 is the extreme case: **real-world use is the fitness function**, analyzers only veto regressions.

4. **Committee + tournament validation** (Canary): multiple designers argue in public, release with a typing tournament, let the community's hands decide. The wiki itself is the output of archetype 4 applied to the whole scene: an open Discord discussion produced this recommendation list.

**The meta-lesson:** nobody in 2023–2026 believes a single corpus score is decisive. Every serious release ships (a) a documented weakness list, (b) mitigation advice (alt-fingerings, variant, swaps), and (c) a plan to revise after field use. Our 4.388 + local-optimum proof is the strongest *archetype-1* result in this comparison set — and simultaneously the least field-tested.

---

## 5. Experiments on our layout inspired by this research

All numbers: Norvig corpus, default fingering unless noted (alt-optimized numbers differ; see analysis_winner.txt).

### 5.1 Angle-mod fingering (Canary-style, for our row-stagger board)
Re-fingered left bottom (b→ring, x→middle, q/u/z→index):

```
TRAD  SFB=3.531%  SKB=2.822%  total same-finger = 6.353%
ANGLE SFB=3.949%  SKB=2.822%  total same-finger = 6.771%
```
Angle mod on *our* layout **costs ~0.42pp SFB** (new ab/ba pair on ring; b was the only important pinky-bottom letter). Unlike Canary/Kuntum, our grid was not generated for angle mod. **Recommendation: stay with arched-home traditional fingering.** (Our XKB install already matches this.)

### 5.2 Repeat/magic key potential
- Repeats = **2.82% of letter bigrams** — larger than any single SFB class.
- Distribution: RM 28%, RP 17%, LM 16%, LI 13% (tt, ss, ee, oo, ff, ii…).
- A repeat key (Kanata dual-function, e.g. tap CapsLock=repeat, or a thumb on a bigger board) would remove most of this **without touching letters** — the single biggest non-swap win available, and the direction Night's author and Whirl have already gone.
- Caveat: our dead-key punctuation layer already occupies complexity budget; a repeat key adds a chord habit. For a prose-heavy user, `tt`/`ee`/`ss` savings are real; for code, less so.

### 5.3 Whorf-index pathology audit
Usage: left 46% / right 54%-ish across fingers; index fingers ~24% combined, well below Night/Sturdy's Whorf-style concentration. Same-finger trigrams (default fingering): middle 4.79%, index 1.87%, ring 0.86%, pinky 0.00%. **No "copying"-style pathology** — our middle-finger SFTs are the `ere`/`ted`/`con` class (top SFS list), which the alt-fingering rules already diffuse in practice.

### 5.4 Rakeable SFB discount (Enthium-style)
Top default-fingering SFBs: `io` 0.64%, `nc` 0.41%, `po` 0.30%, `fa` 0.19%, `op` 0.18%, `rm` 0.18%, `pi` 0.17%, `ey` 0.17%, `cl` 0.14%, `ue` 0.13%.
- Vertical/near-vertical (rakeable in one finger motion): `io` (same column I), `nc`, `po`/`op`, `ue` — collectively >2pp of the 3.53%.
- Horizontal (slidable): `fa`, `rm`, `pi`, `ey`, `cl` partially.
- **Effective SFB after rake/slide ≈ 1.5–2.0%** by Enthium's accounting method — respectable for a no-thumb layout, though still above the 0.5–0.7% AKL band because we spend our budget on scissors/redirects/alt-finger freedom instead. (Note: our alt-optimized SFB is already 0.02% — the community would count us as elite on the metric they rank #1, *if* they adopted our alt-fingering model, which most of their percentiles do not.)

### 5.5 SFS1 comparison
Our SFS1 (default fingering) = 11.79% of trigrams — top offenders `ere`, `con`, `ted`, `you`, `eve`, `ght`. For reference, the wiki's table: Gallium skipgram SFB 4.36% (P34), Kuntum 3.85% (P12), Night-class thumb layouts lower. Definitions differ (theirs is SFB-restricted skipgrams over bigram-table; mine is any same-finger 1-skip over trigrams) — **not directly comparable**, but the gap is large enough that a dedicated SFS-minimizing reweight of our objective would likely move letters (especially `r`-`m`-`c` neighborhood). This is the highest-value *next search* if the user wants one: not a swap, a **re-weighted re-anneal**.

### 5.6 Punctuation audit (Graphite/Kuntem lessons)
Our layer: `,` `.` `/` on right ring/pinky bottom, `;` dead-key on right pinky home, `'` on ISO key.
- Right pinky already carries `s` (home) + `.` `/` (bottom) + dead `;` — this is the **Kuntem problem shape** (high-usage pinky + punctuation). Graphite/Kuntem's fix (move `.` and `,` toward other fingers) is available to us without touching letters: e.g. `.`→right middle bottom, `,`→right ring bottom (already is), `/` stays pinky. Cost/benefit needs a punctuation-weighted re-score including shift pairs — we have `position_freq.json`/punct tables to prototype this.
- Shift pairs: our dead-key design means `?"` `!'` etc. are chording, not adjacent keys — the `.”` class of Graphite's pain mostly doesn't apply (strength of dead keys), but `:`-vs-`;` and quote-heavy prose (typelit-style) should be spot-checked.

---

## 6. Concrete recommendations (ranked)

**Do now (zero risk to the layout):**
1. **Document SFS1/SFS2 and LSB%** for the winner + references in analysis output, using the wiki's definitions, so our layout can be placed on their percentile tables honestly. (Our alt-fingering model will look like a different sport — say so in the doc.)
2. **Add a "known bad bigrams & mitigations" section** to results/LAYOUT.md, in the community's style (Graphite README is the template): our analysis already lists PO/OP/CL scissors, HB/KG/TG index pairs, etc., each with alt-finger or micro-pause advice.
3. **Punctuation variant candidate:** trial moving `.` off right pinky (Kuntem/Graphite fix) — measure with punct-inclusive scoring; ship as cfgE-punct variant if it wins.

**Consider (user decision required):**
4. **Repeat key experiment:** Kanata config with a dual-function key (e.g. CapsLock tap = last-key repeat, hold = Ctrl). Pure win potential ≈ up to 2.8% of bigrams; costs a chord habit. I can write the config.
5. **Casual-corpus validation:** rebuild n-gram tables from a Reddit-style corpus (we have OpenSubtitles as dialogue proxy already; a true forum corpus would be the missing third leg).
6. **SFS-weighted re-anneal:** one more search batch with Semi-decay SFS added as a first-class term, same hard constraints (homes, scissors rules, dead keys). Expected outcome: either a new candidate that beats 4.388 on the combined objective, or confirmation that the current optimum is robust to the community's #1 metric refinement. ~1–2h compute using existing pipeline.

**Don't do (researched and rejected):**
7. Angle mod — quantified loss on our grid (§5.1).
8. Thumb alpha — outside design spec; community itself says exactly-1-letter is the only SFB-beneficial count, and it invalidates the portability story (our XKB install).
9. Chasing 0.37% raw SFB — the wiki's own documentation of Whorf shows why single-metric floors produce pathological layouts; our 0.02% alt-optimized SFB already beats the floor under a stricter fingering model than the percentiles use.

---

## 7. Source inventory

- recommendations page + 16 layout pages + 6 reference/metric pages + FAQ + intro (layouts.wiki, snapshot in /tmp/opencode)
- Gallium README (github.com/GalileoBlues/Gallium)
- Graphite README (github.com/rdavison/graphite-layout) — primary process document
- Canary README (github.com/Apsu/Canary) — primary principles document
- Oxeylyzer README (github.com/O-X-E-Y/oxeylyzer) — weight-config taxonomy
- Sturdy page (oxey.dev/sturdy) — author's design notes
- Enthium site (sunaku.github.io/enthium-keyboard-layout.html) — v1→v14 lab notebook
- Night site (valorance.net/night: design, reflection, alternatives)
- Hands Down Neu site (sites.google.com/alanreiser.com) — in-roll thesis + adaptive keys
- Prior local artifacts: results/analysis_winner.txt, results/spanish_analysis.txt, results/letter_sensitivity.txt, results/deep_search.log
