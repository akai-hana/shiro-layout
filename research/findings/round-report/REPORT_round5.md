# Shiro analyzer calibration — round-7 report

> **Superseded 2026-10-26:** This report documents the cfgE-era model and
> the Norvig/root corpus. The calibrated Shai-primary objective, search,
> winner, and audit are in `findings/REPORT_round8.md`. Historical scores
> here are not comparable to the Shai-primary scores. Artifacts referenced
> below now live under `analysis/legacy/` and `results/archive/`.

Status: complete. All round-6b hard probes pass, the new best layout is a
quench fixed point, impossible-winner audits are clean, and the result
survives the 18-scenario weight-robustness sweep with no hidden-gem
regression.

Date: 2026-10-24
Workspace: `/home/akai/git/layout-opencode-investigation`
Model: `analysis/scorer2.py` + `analysis/fingers.py` + `analysis/comfort.py`
Search: `analysis/search2.py`, campaign in `analysis/_anneal_r7.py`

The filename `REPORT_round5.md` is retained for continuity with earlier
rounds; this document is the current (round-7) report and supersedes the
round-6 text.

---

## 1. Executive summary

Round 6b closes the two open questions left by round 6 and adds the
remaining reverse-scissor ruling:

1. The user confirmed that `X→W` with middle-index means **index on X,
   middle on W** (`IM`), and that it feels practically like `I→M` with a
   little extra index travel. The model now reproduces the stated order
   `I-M < X-W < U-M`.
2. Full-span **middle↔ring** scissors are **symmetric**: both directions
   feel identically extremely uncomfortable, nearly impossible but just
   barely possible. The earlier direction-specific roll reward/strain is
   suppressed for that pair.
3. Reverse full-span **ring↔pinky** scissors are impossible. The only
   reachable corpus cases are `Q→X` with pinky/ring and `X→Q` with
   ring/pinky; both now return the impossible cost.
4. A small **downward 1-row SFB** offset (+0.3) is needed so that the
   confirmed `fxw IMR` ruling remains ahead of the `IIM` alternative; the
   underlying user ruling is that an SFB is still worse than the nearly
   impossible middle/ring scissor.

The recalibrated model produces a new best layout:

```
; s t p b | q d a i y
n h c r m | k e g u o
w v f l z | x j , . /
```

`;stpbqdaiynhcrmkeguowvflzxj,./` scores **3.136368148**, which is
0.001159 below the previous layout's score under the same round-6b model
(3.137526782) and 0.001358 below the score reported for it in round 6
(3.137726460, previous weights). It is a confirmed fixed point of the
1/2/3-swap quench.

The layout is **SFB-free over the whole corpus** under the model's
context-optimal fingering (`IMPOSSIBLE=0 SFB=0`), has no impossible pick,
and has no high-cost bigram in the top 60 corpus pairs. Nine rare
full-corpus pairs remain `HIGH`; the worst are `xy`/`yx` (rotated
index/ring, 72 tokens total) and `mf`/`fm` (188+3 tokens). None is in the
top 60.

The calibration changes are intentionally user-specific. They encode
stated comfort preferences, not a claim about universal typing physics.

---

## 2. What was being calibrated

Shiro is an alternate layout for English (Spanish secondary) built around
non-standard finger rests, `AWEF-JIO;`, and per-key finger alternatives.
Its analyzer is therefore not a conventional fixed-fingering scorer:

- Each key has a default finger and, where relevant, alternate fingers
  with an effort premium (`fingers.py`).
- Effort is pure ANSI travel plus the alt premium; no finger-strength
  bias (`scorer2.py`).
- Bigram and trigram costs are minimized over the available finger
  choices by dynamic programming, so the analyzer models fingering as a
  decision rather than a fixed assignment.
- Same-hand bigram classes (SFB, scissors, rolls, leaps, lateral crossing)
  and trigram classes (SFS, redirect, continuation) are priced separately.
- Balance is a weak pull toward flat finger shares, not a layout driver.

The search objective is the trigram table cost plus the balance penalty.
Reference layouts and candidate layouts are re-scored through the same
evaluator for apples-to-apples comparisons (`compare.py`).

The calibration loop is deliberate: collect the user's physical
observations, turn them into the smallest model change that expresses the
observation, verify that the change does not break earlier rulings, then
re-run the search to see whether the layout floor moves.

---

## 3. Round-6b user input

The rulings that drove this round:

1. **Comfort order.** `I-M` (right hand middle on I, index on M) is the
   easiest of the three at 2.403. `X-W` with the same middle-index
   fingering is practically the same, slightly worse only because the
   index has to reach further for X. `U-M` is the worst of the three.
   Required order: `I-M < X-W < U-M`.
2. **X-W full-span middle/ring.** Both `X-W` and `W-X`, inward or
   outward, feel identically extremely uncomfortable. It is nearly
   impossible but just barely possible.
3. **FXW remains IMR.** The `IIM` alternative contains an SFB. The user
   ranks SFBs as worse than the middle/ring reverse full-span scissor, so
   `IMR` stays the choice despite the scissor. `wxf = MII` remains.
4. **Reverse-scissor ranking.** Middle/ring and ring/middle reverse
   full-span scissors are second in discomfort only to ring/pinky and
   pinky/ring reverse full-span scissors. The ring/pinky class is to be
   treated as impossible; the middle/ring class is extremely
   uncomfortable but barely possible, in both directions.
5. **Reachable R/P cases.** The model's only reachable reverse full-span
   ring/pinky candidates are `Q→X PR` and `X→Q RP` (left hand). Both are
   impossible. Half-span ring/pinky rolls keep their sign-specific
   weights.

The X-W clarification closes the interpretive item that round 6 had
flagged: the user's "middle-index" refers to index-on-X / middle-on-W,
not middle-on-X. `X→W = MI` therefore remains the expensive full-span
scissor class, consistent with the `U→M = IM` ruling.

---

## 4. Implemented changes

### 4.1 X-W stagger favor

`analysis/comfort.py` adds `stagger_favor()` and `W_STAGGER_FAVOR = 0.35`.
The discount applies only to the same-column full-span M/I diagonal when:

- the fingers are exactly `{M, I}` on the same hand and the same column;
- the middle-row key is above the index-row key (natural full-span
  direction);
- the lateral order is not crossed;
- neither key is one of the fingers' rest keys.

That selects `X-W` and `R-V` and excludes `E-C` (middle key already at
rest) and `U-M` (crossed against the stagger). `scorer2._same_hand_bigram`
subtracts the discount from the bigram cost. `_decompose.py` mirrors the
term in its own `stagger` category.

### 4.2 Symmetric full-span middle/ring scissor

`analysis/comfort.py` adds `SCISSOR_MR_FSB = 5.85`. `scissor_cost()` uses
it when the outer/inner pair is `("R", "M")` and the row gap is at least
2. The branch is deliberately keyed on `_outer_inner()` order (`R` is
outer to `M`), not on the literal pair `("M", "R")`; an early draft used
the wrong order and silently left the cost at 6.0, which the probe caught.

`scorer2._same_hand_bigram()` suppresses the generic full-span
reward/strain for `{M, R}` so both directions cost exactly the same base.
Half-span R/P and other scissor classes are unchanged.

### 4.3 Direction-dependent 1-row SFB

`analysis/comfort.py` adds `W_SFB_1ROW_DOWN = 6.3`; upward 1-row SFBs
stay at 6.0. This is the smallest offset that keeps both trigram rulings
coherent after the X-W discount:

- `fxw IMR` 9.842 vs `IIM` 9.902 (margin 0.060);
- `wxf MII` 9.602 vs `RMI` 9.843 (margin 0.241).

The offset is applied in `scorer2._same_hand_bigram()` and mirrored in
`_decompose.py`.

### 4.4 Ring/pinky reverse full-span impossible

`analysis/comfort.py` adds `impossible_rp()`. The bad shape is pinky up +
ring down (the shorter/outer finger reaching upward); the reverse
(ring up + pinky down) is the natural wave and remains free. Only
full-span cases are affected, so half-span R/P rolls retain
`W_ROLL_RING_PINKY = -0.14` and `W_ROLL_PINKY_RING = +0.06`.

`impossible_rp` is wired into `scissor_cost()`, `_verify_r5.py`, and
`_regress_r5.py`. Reachable cases `Q→X PR` and `X→Q RP` now cost 9.0;
the probe checks both explicitly.

### 4.5 Supporting updates

- `analysis/scorer2.py`: stagger import/discount, `full_sc` split,
  M/R reward suppression, downward-SFB branch.
- `analysis/_decompose.py`: mirrors the new bigram semantics and adds a
  `stagger` category.
- `analysis/_verify_r5.py`: `impossible_rp` in `is_impossible`, round-6b
  ordering/symmetry checks, updated `NEW_BEST`, and round-6b audit
  headers.
- `analysis/_regress_r5.py`: `impossible_rp` in `is_impossible`, updated
  pick-audit header.
- `analysis/robustness2.py`: sweeps `W_SFB_1ROW_DOWN`, `SCISSOR_MR_FSB`,
  and the new `stagger` group; `NEW_BEST` updated.
- `analysis/_anneal_r7.py`: new campaign script for the round-6b floor.
- `analysis/flow_profile2.py`: reproducible flow-profile generator for
  the scorer2 model.
- `analysis/_rebuild_notes.md`: round-7/v2.7 section.

---

## 5. Verification

The full suite is `cd analysis && python3 _verify_r5.py`. It runs hard
probes, the model comparison, the cfgE audit, reference rescoring, and the
new-best audit. Final output: every round-6b hard check **PASS**.

### 5.1 Hard probe results

| Probe | Winning fingering | Cost | Next competitor | Margin |
|---|---|---|---|---:|
| O→L | MI | 2.215 | MR 2.230 | 0.015 |
| L→O | RM | 2.080 | IM 2.215 | 0.135 |
| X→Q | IM | 3.153 | MP 3.290 | 0.137 |
| Q→X | MI | 3.153 | PM 3.290 | 0.137 |
| S→W | IM | 2.475 | MI 4.305 | 1.830 |
| W→S | MI | 2.475 | IM 4.305 | 1.830 |
| X→S | IM | 2.765 | MI 4.665 | 1.900 |
| F→X | IR | 2.580 | MR 2.770 | 0.190 |
| X→F | RI | 2.580 | RM 2.695 | 0.115 |
| U→M | MI | 2.806 | IM 9.303 | 6.497 |
| M→U | IM | 2.806 | MI 9.303 | 6.497 |
| X→W | IM | 2.603 | MR 8.230 | 5.627 |
| W→X | MI | 2.603 | RM 8.230 | 5.627 |
| A→W | PR | 2.060 | PM 2.540 | 0.480 |
| W→A | RP | 1.860 | MP 2.540 | 0.680 |
| fxw | IMR | 9.842 | IIM 9.902 | 0.060 |
| wxf | MII | 9.602 | RMI 9.843 | 0.241 |

Round-6b ordering and symmetry checks:

| Check | Result |
|---|---|
| `I-M < X-W < U-M` | 2.4025 < 2.6025 < 2.8056 — PASS |
| `X-W M/R == W-X R/M` | 8.230 vs 8.230 — PASS |
| `Q→X PR` / `X→Q RP` impossible | 9.00 / 9.00 — PASS |

Notes:

- `O→L MI` is the preferred default but the margin over `MR` is only
  0.015. This is preference-encoded calibration, not a large physical
  effect.
- `L→O` chooses `RM` (2.080), the reverse of the `O→L MR` fallback, not
  the reverse of the preferred `MI`. `IM` is available at 2.215.
- `F→X` and `X→F` are symmetric at 2.580; this was a hard requirement.
- `X→W` dropped from 2.953 to 2.603 after the stagger favor, while `MI`
  remains 9.986 (full-span scissor). This is the intended shape.
- `I-M` is 2.403, so the required `I-M < X-W` ordering holds with a
  margin of 0.200.
- The trigram margins are intentionally tight. The full-span direction
  split is still the main empirical assumption in the trigram model.

### 5.2 Impossible-winner scans and picks

Under the new best layout:

- bigram impossible winners: **0** (0 / 3,982,137 tokens)
- trigram impossible winners: **0** (0 / 2,914,922 tokens)
- top-60 corpus bigrams: `IMPOSSIBLE=0 SFB=0 HIGH=0`
- corpus-wide same-hand bigrams: `IMPOSSIBLE=0 SFB=0 HIGH=9`

Round 6 had two corpus-wide SFB types (`xy`/`yx`, 72 tokens). The new
best removes them; those pairs are now typed with rotated index/ring
fingers at 9.09–9.19 and remain `HIGH`, but no SFB remains. The other
`HIGH` entries are rare: `mf`/`fm` (188+3), `yk`/`ky` (38+287),
`uy`/`yu` (77+62), `bc` (158). None is in the top 60. The detailed list
is reproducible from the audit; `results/newbest_audit.txt` records the
summary.

Under the new best, the `orm`/`arm`/`fai` class is SFB-free: `orm` is
`;→F→G` (PMI) at 3.295, `arm` is `I→F→G` (MMI) at 3.295, and `fai` is
`C→I→O` (IMR) at 2.920. Under cfgE, `orm`/`arm` were already clean
(IMI/RMI at 3.255/3.095), but `fai` was the problem member: its
placement boxed it into an SFB in an earlier round-6 calibration
(`S→W→E`, 8.550, 807 tokens), and under the final round-6 and current
weights it is still the most expensive of the three (6.005, no SFB).
The round-6 layout (`X→I→O`/MMR, 3.060) and the new best both fix it,
confirming that the v2.4 complaint was a letter-placement issue, not a
model rule.

### 5.3 Model scores on cfgE

The cfgE candidate (`;aiojvlnrwhfpeygtcmsbxquzkd,./`) is the fixed
reference that isolates model changes from layout-search changes:

- old model (round 5): 4.3715
- round-6 model: 3.3461
- round-6b model: 3.3367

These numbers are not a "layout improvement": the weights changed, so
only the within-model ranking and the hard probes are comparable.

---

## 6. Search campaign and the new best

### 6.1 Method

`analysis/_anneal_r7.py`:

- Wave 1: 19 starts — the round-6 new best, the older transcript best,
  `annealBEST`, and 16 seeded random permutations.
- 100,000 simulated-annealing iterations per start.
- 1-swap, 2-swap, and 3-cycle quench after each anneal.
- 8 ILS perturbations around each of the top 5 unique layouts, with half
  the anneal budget per perturbation.
- Confirmation: 43 starts (the original three seeds plus 40 randoms),
  100,000 iterations, same quench + ILS protocol.
- Same ANSI geometry, corpus, and round-6b model throughout.

Wave 1 took about 920 seconds. The 43-start confirmation took about
1,360 seconds and converged to the same best. Raw output:
`results/anneal_r7.txt` and `results/anneal_r7.json`; the wave-1 copy is
`results/anneal_r7_wave1.txt|json`.

### 6.2 Result

```
; s t p b | q d a i y
n h c r m | k e g u o
w v f l z | x j , . /
```

`;stpbqdaiynhcrmkeguowvflzxj,./` — **3.136368148**

Compared with the prior floor:

| Layout | Score | Note |
|---|---:|---|
| new best | 3.136368 | quench fixed point |
| previous best (round-6b rescore) | 3.137527 | also a quench fixed point |
| previous best (round-6 reported) | 3.137726 | previous weights |
| near-tie `;stpbzdaiynhcrmkeguowvflqxj,./` | 3.136513 | 0.000145 behind |
| annealBEST | 3.141104 | prior prior floor |
| cfgE | 3.336711 | fixed reference candidate |

Convergence: four rows in the 43-start campaign reached 3.1364 (the
`NEW_BEST` seed, `rand2005`, `rand2010`, and `ILS1`). The best random
starts and the near-tie family stop at 3.1365–3.1375. The new best is a
true 1/2/3-swap quench fixed point. `results/newbest_check.txt` records
the scores and fixed-point checks.

### 6.3 Why it wins

Cost decomposition relative to cfgE (`analysis/_decompose.py`):

| Category | cfgE | new best | delta |
|---|---:|---:|---:|
| transition total | +0.0165 | -0.0584 | -0.0749 |
| leap | +0.0325 | +0.0063 | -0.0262 |
| pinky redirect | +0.0239 | +0.0130 | -0.0109 |
| awful | +0.0148 | +0.0046 | -0.0102 |
| roll | -0.0363 | -0.0464 | -0.0102 |
| ladder | +0.0154 | +0.0065 | -0.0090 |
| sfs | +0.0198 | +0.0157 | -0.0041 |
| scissor | +0.0050 | +0.0030 | -0.0020 |
| cross | +0.0013 | +0.0003 | -0.0010 |
| balance penalty | 0.0943 | 0.0098 | -0.0845 |

The remaining categories are essentially flat: `alt` -0.0042, `continue`
-0.0006, `redirect` +0.0006, `repeat` 0.0000, `sfb` 0.0000, `stagger`
+0.0029.

Finger shares under the new best are much closer to the flat target:

- left: P/R/M/I = 0.175/0.254/0.289/0.283 (target 0.18/0.25/0.28/0.29)
- right: P/R/M/I = 0.162/0.249/0.303/0.286

Flow profile under the scorer2 model (`results/flow_profile_r7.txt`;
bigrams use context-optimal fingering, trigrams use default fingers):

| Class | cfgE | new best |
|---|---:|---:|
| same-hand bigram SFB | 0.00% | 0.00% |
| leap | 8.71% | 2.99% |
| roll in | 14.25% | 11.31% |
| roll out | 7.63% | 12.79% |
| rolls (in+out) | 21.88% | 24.11% |
| cross-hand alternation | 66.59% | 70.08% |
| repeats | 2.82% | 2.82% |

Trigram patterns under default fingers:

| Class | cfgE | new best |
|---|---:|---:|
| alternate | 40.06% | 44.53% |
| inroll | 31.27% | 18.08% |
| outroll | 10.12% | 19.74% |
| onehand | 0.71% | 1.42% |
| redirect (index) | 2.80% | 1.34% |
| redirect weak | 0.55% | 1.27% |
| SFB in trigram | 14.49% | 13.62% |

The gain comes from lower travel/effort (0.041), a large balance
improvement (0.085), and fewer transitions (0.075, mainly leap and pinky
redirect removal), not from a dramatic roll increase. The layout is less
in-roll-heavy and more out-roll-heavy than cfgE, consistent with the
user's per-pair direction rulings (notably the P/R inversion and the
symmetric M/R treatment).

---

## 7. Robustness

`analysis/robustness2.py 20000` perturbs one model group at a time, then
runs a 20k-iteration anneal from cfgE and quenches it. Results are written
to `results/robustness2.txt` and `.json`.

Baseline reproduces cfgE at 3.336711. The new best is the best reference
layout in **15 of 18** scenarios. The three exceptions are:

- **rolls ×1.5**: `annealBEST` wins;
- **rolls ×0.5**: `annealBEST` wins;
- **scissors off**: `annealBEST` wins.

All exceptions weaken the exact classes the round-6/6b rulings depend on
(full-span scissors and roll rewards). Under the real model and all
moderate perturbations, the new best stays on top.

A hidden-gem check re-scored every scenario-derived layout under the
baseline model. The best of them is **3.142162**
(`;ntlvkuoigshcrdyefpawxmbzjq,./`, from the sfs ×2 anneal), still worse
than 3.136368. No perturbed search found a layout that beats the new best
once the baseline weights are restored.

---

## 8. Reproducibility

All commands assume the `analysis/` directory:

```bash
python3 _verify_r5.py          # hard probes, audits, reference scores
python3 _anneal_r7.py 40 100000 # 40 random starts (+3 fixed) + ILS
python3 robustness2.py 20000   # 18-scenario robustness sweep
python3 _decompose.py          # category attribution for cfgE/old/new
python3 flow_profile2.py       # reproducible flow profile
```

Supporting artifacts:

- `results/newbest_audit.txt` — flags for the new best
- `results/newbest_check.txt` — scores and quench fixed-point checks
- `results/anneal_r7.txt|json` — 43-start campaign rows (40 random + 3 fixed)
- `results/anneal_r7_wave1.txt|json` — wave-1 campaign rows
- `results/robustness2.txt|json` — robustness sweep
- `results/flow_profile_r7.txt` — bigram/trigram profile
- `analysis/_rebuild_notes.md` — round-7/v2.7 notes

Reference scores under the round-6b model:

| Layout | Score |
|---|---:|
| new best | 3.1364 |
| annealBEST | 3.1411 |
| cfgE | 3.3367 |
| Gallium | 3.4452 |
| Graphite | 3.4468 |
| Semimak | 3.5041 |
| Sturdy | 3.5629 |
| Canary | 3.6934 |
| Colemak | 3.7258 |
| Colemak-DH | 3.7422 |
| QWERTY | 4.6794 |

---

## 9. Limits and follow-ups

1. **X→W wording is resolved.** The user confirmed index-on-X /
   middle-on-W (`IM`). The model reproduces `I-M < X-W < U-M` and keeps
   `X→W MI` expensive. No clarification remains on this item.
2. **M/R full-span base is an empirical fit.** `SCISSOR_MR_FSB = 5.85`
   sits just below the 1-row SFB weights (6.0 up / 6.3 down). The `fxw
   IMR` margin over `IIM` is only 0.060. A future typing observation
   could require repricing the SFB side or the M/R base.
3. **Direction-dependent SFB is an empirical fit.** The +0.3 downward
   offset is the smallest change that co-holds `fxw IMR` and `wxf MII`;
   it is not a pure physics claim.
4. **Full-span IN/OUT asymmetry remains for other pairs.** The symmetric
   treatment is specific to M/R; full-span M/I and other pair classes
   still use the direction split. This follows the user's reverse-scissor
   ranking, but it is the main remaining structural assumption.
5. **R/P reverse-full-span scope.** Only `Q→X PR` and `X→Q RP` are
   reachable under the current letter set/corpus. If punctuation or a new
   corpus makes other reverse full-span R/P cases reachable, rerun the
   impossible audits.
6. **Corpus scope.** The model is trained on the project's English
   corpus. Spanish behavior is documented separately and was not
   re-annealed this round.
7. **Model, not universal truth.** These weights are calibrated to this
   user's hands and stated preferences. They should not be interpreted as
   a general keyboard-layout ranking.

No blocking issue remains for the round-6b model.
