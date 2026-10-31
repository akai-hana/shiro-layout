# Round 8 — calibrated Shai-primary re-anneal and audit

Status: provisional result ready for review. All hard questionnaire probes
pass, the winner is a quench fixed point under 1-swap / 2-swap / 3-cycle
moves, and four independent campaigns found no better layout. One comfort
question remains: the winner uses the very frequent `th` bigram as a mild
adjacent ring/middle scissor.

Date: 2026-10-26
Workspace: `/home/akai/git/layout-opencode-investigation`
Model: `analysis/scorer2.py` + `analysis/fingers.py` + `analysis/comfort.py`
Search: `analysis/anneal_shai.py`
Audit: `analysis/audit_shai.py`
Analysis: `analysis/analyze_shai.py`

This supersedes the cfgE-era round-7 report for the Shai-primary objective.
Historical scores in `findings/REPORT_round5.md` are not comparable to the
scores below.

---

## 1. Result

Layout string (30 main keys, `KEY_ORDER`):

```text
;tspbxgoiwndhrlyecuafkvmzjq,./
```

Grid (QWERTY key names are physical positions):

```text
        L hand            R hand
top :   ; t s p b        x g o i w
home:   n d h r l        y e c u a
bot :   f k v m z        j q , . /
```

Scores (lower is better):

| metric | score |
|---|---:|
| composite | **3.165795758** |
| primary (Shai) | **3.165796149** |
| secondary (Spanish) | 3.274330031 |
| raw trigram cost | 3.160174631 |
| balance penalty | 0.005621127 |

Primary raw = 3.160174541; Spanish raw = 3.169171737. The primary-sacrifice
setting is `1e-05`; the Spanish scale is `0.000623095`.

This is an unconstrained re-anneal under the calibrated Shai-primary
objective. It is not a small edit of the shipped cfgE layout and it does not
preserve cfgE's all-vowels-left design. The rest keys now hold
`n t s r / e o i a`, i.e. the high-frequency letters sit on the arched rest
position.

---

## 2. Objective and corpus

- Primary corpus: `corpus/shai/` — official Shai iWeb letters-only corpus.
  - `418,381,828` letters
  - `327,253,737` bigrams
  - `240,524,895` trigrams
  - `91,128,091` runs
  - source: `corpus/shai/iweb-corpus-samples-cleaned.txt`, MD5
    `1d19b83e21d0a681ff776bb8e8a57e9d`
  - construction: lowercase, maximal `[a-z]+` runs; all non-letters break
    runs, so no punctuation-crossing pairs are fabricated.
- Secondary corpus: `corpus/spanish/`, used as a near-tie sanity check only.
- Objective: minimize primary score; allow up to `1e-05` primary regression
  for a Spanish improvement, with one full Spanish score unit weighted at
  `0.000623095` primary units.
- The corpus builder can verify the current data with:
  `python3 analysis/build_shai_corpus.py --check`

The old report's `3.136368` was produced with the Norvig/root corpus and an
earlier model. It is not a score for this corpus or this model.

---

## 3. Calibration encoded this round

The questionnaire was answered against the old layout
`;trdpzgaifsmcnlxehuowkvbjqy,./`. Rules are encoded by **physical key**, not
by the old letter placements. After re-annealing, an old example letter may
be on a different physical key, so the probes below are the authoritative
checks. For example, `O->L` means physical O -> physical L; in this layout
the O key holds `i` and the L key holds `u`.

Implemented calibrations:

| rule | implementation |
|---|---|
| same-key repeats | forced to one finger; `W_REPEAT=0.6` |
| one-row same-finger sequences | `W_SFS_1U=2.50` |
| full-span M/R reverse scissors | `W_CR_REVERSE=3.60`; symmetric for `{M,R}` |
| K ring alternate | `0.30` premium, applies to K-J-M |
| alt-context gating | `W_ALT_CONTEXT=12.0` |
| L index | only when the physical O key is in the trigram |
| G middle | always blocked |
| V middle | blocked with G |
| C middle | blocked with R |
| J middle | blocked with T / I / G |
| M middle | blocked with J |

The earlier “C middle without R” audit gate was inverted; it is now the
correct “C middle with R” violation check and reports no violations.

A subtle case: direct `M-J` bigram selects J middle, but the ruled
`M-J-I` trigram selects index/index for the first two strokes. The audit
checks the trigram, not the direct bigram. Likewise, `K-I` has no explicit
ring requirement: the top choice is K index at `1.965` versus K ring at
`2.000`, a near tie.

---

## 4. Search campaign

All campaigns started from the current winner and used the calibrated
objective with the Shai corpus primary. `ILS` is the number of iterated
local-search rounds after each anneal.

| campaign | seed | runs | iters | ILS | time | best |
|---|---:|---:|---:|---:|---:|---:|
| A | 3041 | 8 | 40,000 | 4 | 506 s | 3.165795758 |
| B | 9221 | 8 | 60,000 | 4 | 653 s | 3.165795758 |
| C | 7717 | 12 | 60,000 | 4 | 993 s | 3.165795758 |
| D | 18181 | 12 | 60,000 | 4 | 976 s | 3.165795758 |

Totals: 40 random starts plus seven fixed seeds, 80 scored anneal/ILS runs.
No run beat the winner; several independent random starts converged back to
it. The winner was also the top result of the earlier 367-candidate scan and
of campaigns A/B.

Closest alternatives (same objective):

| rank | layout | composite | primary | Spanish |
|---:|---|---:|---:|---:|
| 1 | `;tspbxgoiwndhrlyecuafkvmzjq,./` | **3.165795758** | 3.165796149 | 3.274330031 |
| 2 | `;nshvkgoifrlctwyeupadxbmzjq,./` | 3.168539438 | 3.168539577 | 3.352533199 |
| 3 | `;trhxzgoipndlswyeucafkbmvjq,./` | 3.169045361 | 3.169045569 | 3.242832347 |
| 4 | `;nshvkgoiflrctwyeupadxbmzjq,./` | 3.169506529 | 3.169507013 | 3.339016638 |
| 5 | `;trhxzmoiwndlspyecuafkbgvjq,./` | 3.170466357 | 3.170466796 | 3.242405889 |

The margin to rank 2 is `0.002744`, so this is a genuine best-of-campaign
result, not a tie broken by noise.

---

## 5. Questionnaire and corpus audit

Hard probes (`analysis/audit_shai.py`): all PASS.

```text
repeated keys use one finger          26/26
O->L                                   MI
K-J-M                                  RMI
F-J-M                                  IMI
L->O                                   top RM; best L-index 2.215
M-J-I                                  IIM
H-T-J exp blocks J middle              III
I-U-J age blocks J middle              RMI
V-G-J forces V/G index                 III
C-O-R vid endpoints index              IRI
R-O-C div endpoints index              IRI
L index only with O key (bigram)       all non-O bigram neighbours
```

The L-index probe is bigram-only; the corpus-wide trigram gate scan also
requires the physical O key anywhere in the trigram and reports no
violations.

Corpus-wide scan (Shai, weighted):

| item | value |
|---|---:|
| bigram types / tokens | 676 / 327,253,737 |
| alt-fingered bigram tokens | 34,804,916 (10.64%) |
| SFB types / tokens | 6 / 116,058 (0.04%) |
| impossible bigrams | 0 |
| SFS types / tokens | 514 / 1,743,852 (0.73%; far 98,915) |
| half-SFB types / tokens | 473 / 415,132 (0.17%) |
| alt-gate violations | 0 |
| actual alt strokes | 56,346,459 / 721,574,685 (7.81%) |

Top residual SFBs:

| pair | count | motion | cost |
|---|---:|---|---:|
| `lm` | 83,892 | G->V, index/index | 8.74 |
| `ml` | 17,137 | V->G, index/index | 8.44 |
| `kt` | 11,748 | X->W, ring/ring | 10.43 |
| `tk` | 2,628 | W->X, ring/ring | 10.43 |
| `yj` | 517 | H->N, index/index | 8.74 |
| `jy` | 136 | N->H, index/index | 8.44 |

Top SFS:

| trigram | count | keys | fingers | distance |
|---|---:|---|---|---:|
| `mpl` | 245,302 | V R G | II | 1.12 |
| `ely` | 190,661 | J G H | II | 1.00 |
| `wou` | 179,366 | P I L | RR | 1.25 |
| `mil` | 163,677 | V O G | II | 1.12 |
| `mal` | 117,714 | V ; G | II | 1.12 |
| `lem` | 94,722 | G J V | II | 1.12 |
| `owi` | 69,778 | I P O | MM | 1.00 |
| `wea` | 57,215 | P J ; | RR | 1.03 |
| `lim` | 56,893 | G O V | II | 1.12 |
| `mpr` | 55,440 | V R F | II | 1.12 |

Most SFS volume is the forced `l`/`m` placement on G/V, both index keys.
The same placement forces the `lm`/`ml` SFBs. The `mpl`/`mil`/`mal`/`lem`
cluster is the price of keeping that high-frequency pair on the index.

Alt use is load-bearing, not cosmetic:

| key | alternate share of trigram strokes |
|---|---:|
| T | 56.8% |
| R | 50.0% |
| K | 36.0% |
| C | 23.8% |
| Y | 17.2% |
| B | 15.0% |
| S | 11.6% |
| D | 11.4% |
| O | 10.7% |
| V | 8.9% |
| U | 6.5% |
| L | 6.1% |

Balance is strong:

| hand | pinky | ring | middle | index |
|---|---:|---:|---:|---:|
| left | 17.6% | 26.4% | 28.8% | 27.2% |
| right | 17.0% | 25.3% | 27.8% | 29.9% |

---

## 6. Corpus sensitivity

In the original campaign the winner was rank 1/16 on every tested corpus
except Spanish, where it was rank 2/16 behind `old_shai_alt1`.

| corpus | winner | score | current | rank | delta vs best |
|---|---|---:|---:|---:|---:|
| Shai | current | 3.165796 | 3.165796 | 1/16 | 0.000000 |
| Spanish | `old_shai_alt1` | 3.180372 | 3.274330 | 2/16 | +0.093958 |
| chat_write | current | 3.184173 | 3.184173 | 1/16 | 0.000000 |
| code | current | 3.262686 | 3.262686 | 1/16 | 0.000000 |
| word | current | 3.165796 | 3.165796 | 1/16 | 0.000000 |
| wiki | current | 3.165409 | 3.165409 | 1/16 | 0.000000 |
| norvig_root | current | 3.168190 | 3.168190 | 1/16 | 0.000000 |

The auxiliary corpora (`chat_write`, `code`, `word`, `wiki`, and the Norvig
root corpus) are no longer shipped; those rows are retained from the original
campaign. The current `audit_shai.py` re-scores the shipped Shai and Spanish
corpora only.

Spanish trade-off, explicitly: `old_shai_alt1`
(`;tncbxgaifsdprlkehuowzvmjqy,./`) wins Spanish at `3.180372` but costs
`0.0275` primary. Under the current `1e-05` sacrifice rule the primary
winner is correct. If Spanish should be equal priority, use the Spanish
winner instead.

---

## 7. Reference comparison under the calibrated model

All scores are on the Shai primary corpus under the round-8 model. They are
not comparable to the historical Norvig/root scores.

| layout | primary score |
|---|---:|
| **current Shai winner** | **3.165796** |
| round-7 cfgE-era winner `;stpbqdaiynhcrmkeguowvflzxj,./` | 3.225648 |
| round-6 winner `;stckqdaiwnlprhyeguomfvbzxj,./` | 3.205997 |
| old Shai lead `;trdpzgaifsmcnlxehuowkvbjqy,./` | 3.222834 |
| old Shai alt1 `;tncbxgaifsdprlkehuowzvmjqy,./` | 3.193260 |
| old Shai alt2 `;thsbxcoiwndprlyeguafkvmzqj,./` | 3.179118 |
| chat best `;thskxpoiyrdgnwfeucalzmbvqj,./` | 3.206501 |
| shipped cfgE `;aiojvlnrwhfpeygtcmsbxquzkd,./` | 3.454663 |
| QWERTY | 4.704425 |
| Graphite | 3.486794 |
| Colemak | 3.670764 |
| Colemak-DH | 3.688144 |
| Canary | 3.678354 |
| Sturdy | 3.557625 |
| Gallium | 3.473538 |
| Semimak | 3.554755 |

Relative to round 7 the calibrated Shai-primary gain is `0.059852`; relative
to the shipped cfgE reference it is `0.288868`.

---

## 8. Open questions and limitations

1. **`th` is the main comfort question.** In the winner `t` is on W (left
   ring) and `h` is on D (left middle): physical W->D, a mild adjacent
   ring/middle scissor cost of `0.21`. It occurs `10,712,957` times and is
   the largest contributor to `scissor_MR = 4.24%`. The model prices this
   mild class cheaply. If it feels bad to you, the next step is to raise the
   adjacent R/M scissor weight and re-anneal; the layout would likely move
   `t`/`h`.
2. **Residual forced SFBs.** `lm`/`ml` are the forced G/V index pair
   (101,029 tokens). `kt`/`tk` (14,376 tokens) are a K-alternate versus T-ring
   collision; the optimizer chose the SFB over the full-span scissor.
   `yj`/`jy` (653) are forced by J having no middle alternate. These are all
   below 0.04% of bigrams but are real.
3. **`K-I` is near-tied.** K index `1.965` versus K ring `2.000`; no explicit
   ring requirement was added. Your note that the flow is mildly
   uncomfortable is not strongly represented. A penalty would also affect
   K-J-M.
4. **Spanish is rank 2.** The winner trades `0.093958` Spanish score for the
   primary win. That is intentional under the current sacrifice rule.
5. **Design branch.** This result does not preserve cfgE's all-vowels-left
   arrangement. If that arrangement is a hard requirement rather than a
   previous preference, the correct next step is a constrained search, not
   adoption of this winner.
6. **Model, not measurement.** The layout is optimal under the questionnaire-
   calibrated model, not proven optimal for your hands. Global optimality is
   not established; the result is a strong empirical fixed point.

---

## 9. Reproduce

```bash
python3 analysis/build_shai_corpus.py --check
python3 analysis/anneal_shai.py \
  --runs 12 --iters 60000 --ils 4 --seed 7717 \
  --seed-layout ';tspbxgoiwndhrlyecuafkvmzjq,./' \
  --out results/round8/anneal_shai_c.txt \
  --out-json results/round8/anneal_shai_c.json
python3 analysis/audit_shai.py
python3 analysis/analyze_shai.py
```

Artifacts:

- `results/round8/anneal_shai_a.txt|json` through
  `results/round8/anneal_shai_d.txt|json`
- `results/round8/audit_shai.txt|json` — full hard probes and corpus scan
- `results/round8/analysis_shai.txt|json` — scores, categories, worst pairs,
  usage, and per-letter alt guide
- `results/LAYOUT.md` — current provisional guide
- `results/archive/LAYOUT_cfgE.md` — shipped cfgE guide, retained as
  historical baseline
- `findings/REPORT_round5.md` — historical cfgE-era report, now superseded
  for the Shai-primary objective

---

## 10. Verdict

Use `;tspbxgoiwndhrlyecuafkvmzjq,./` as the provisional Shai-primary result.
It is the clear campaign winner, passes every hard questionnaire probe, has
no impossible pairs, and is rank 1/16 on the shipped Shai corpus (rank 2/16
on Spanish); it also won the auxiliary corpora in the original campaign. Keep
cfgE as the known-good shipped layout until the `th` adjacent-scissor
question is resolved. No commit was made.
