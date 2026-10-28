# Shiro pattern analyzer — merged with round 8, updated with your answers of 2026-10-27 to 2026-10-29

The pattern model (`comfort_geo.py`: key geometry + the arched rest `AWEF / JIO;`) merged with your round-8 work
(`audit_shai.py` probes, `scorer2.py` alt-context gates, `W_CR_REVERSE`, `W_SFS_1U`, repeats), then corrected with your answers.

## 1. What your latest answers (2026-10-29) changed

| you said | what I did |
|---|---|
| A-S pinky-ring; K-; middle-pinky; A-E pinky-middle; ;-K pinky-middle; O-J ring-index; F-S index-ring; S-F ring-index; X-A index-ring | all enforced. **A-S replaces my earlier "ring on A, middle on S" (from your 2026-10-27 answer)**; the old two rulings are gone. A→S PR 0.57 (RM 0.80), S→A RP 0.28. The rest were already enforced. |
| ;-U pinky-middle; ring-middle on ;-U is borderline impossible | ;→U PM 0.50; ring+middle priced ≥ 5 in both orders. New general pattern: **two adjacent fingers spread more than 2 key widths beyond their rest spacing cost +6** (`SPREAD_LIM`, `SPREAD_IMP`). It also applies to other pairs that wide, e.g. ring on `;` with middle on U or Y. |
| T-F index-middle and vice versa | T→F IM, F→T MI. |
| ;-L pinky-ring | ;→L PR 0.57, L→; RP. |
| J-Y and Y-J right-index / left-middle | **not enforced, see section 4.** |

Also, because you wanted "those preferences only": the previous review's largest disagreements with your shipped scorer (A-S, K-;, A-E, ;-K, O-J, F-S, S-F, X-A, ...) are your own rulings, not model errors.

## 2. Result

| | |
|---|---|
| rulings in `rulings.py` | 168 (166 enforced; the 2 J-Y / Y-J rulings are held out) |
| passed | **166 / 166** enforced |
| probes + hard checks in `audit_geo.py` | **37 / 37** |
| your shipped `scorer2.py` on the same rulings | 123 / 168 |
| agreement with `scorer2.py` on same-hand bigram mass (round-8 winner layout) | **87.9 %** (81.6 % before today) |
| table scale vs yours | mean bigram 2.66 vs 2.58, mean trigram 4.63 vs 4.29 |

Where the other 12.1 % comes from: 11.6 % are bigrams one of your rulings speaks about (mostly your explicit answers, which the shipped scorer simply fails; J-Y is among them while it is held out); **0.5 % (26 bigrams) are on pairs no ruling covers**, and in only 5 of those does your shipped fingering cost less than 0.3 more in this model. The biggest unruled ones are tiny (Y→O/O→Y, T→W, E→Z at about 0.02–0.03 % of all bigram mass each).

47 parameters, 12 of them your declared constants, frozen. Fitted values of the patterns added in recent rounds: `e_nd` 0.10, `e_nd_rest` 0.57, `e_ndgap` 0.65, `nd_waive` 0.90, `shift_disc` 0.53, `shift_thr` 0.66, `e_upcol` 0.11, `scis_lat` 1.93, `scis_dir` 0.17. The pinky/ring scissor strength `a_PR` fell from 0.92 to 0.22 to make home-row pinky-ring pairs (A-S, ;-L) cheap; full-span pinky/ring stays impossible. `IP_SMIN` = 1.9 and `SPREAD_LIM` = 2.0 are constants I chose from your answers.

## 3. What is pattern and what is table

Without the gate table the same model passes 162 of 167 rows (166 rulings + the anchor row); the five failures are the probes that need explicit licence gates (K-J-M, F-J-M, M-J-I, H-T-J, V-G-J). The eight-row table in `gates.py` is unchanged:

| key, finger | allowed / blocked |
|---|---|
| X middle | only with C or A |
| W middle | only with Q, X or S |
| L index | only with O |
| C middle | blocked with R |
| G middle | never |
| V middle | blocked with G |
| J middle | blocked with T, I, G |
| M middle | blocked with J |

Held-out check (`loo3.py`, `loo_warm.log`): leave one family of rulings out, refit warm-started, test the held-out family. **212 / 219** held-out rulings still pass. Not predicted: O-K/K-O, one K-; ruling, one X→A ordering, K-J-M, one leap ruling and the ring→pinky direction. Those remain decisions of yours the pattern only reproduces once you state them. The figure is optimistic (warm start from a solution that already satisfied the held-out rulings); the cold-start test was not re-run. The index/pinky and spread thresholds are constants set from your answers, so those families are not predicted.

## 4. One thing I need you to confirm: J-Y / Y-J

You said "right-index / left-middle and vice versa". Y is on the right hand's side of the keyboard, left of J, so there are two readings:

1. **Same hand, crossed:** right index on J and right middle on Y, so the middle finger sits left of the index. The model prices this as impossible (crossing 1.25 key widths, 12.4). Your old rule "index on X + middle on F crosses 1.5 keys: impossible" puts the limit near that, so enforcing it would need the crossing limit to sit between 1.25 and 1.5, and a fit with it disturbed the M/I ladder and the leap ordering.
2. **Other hand:** right index on J and the **left** middle finger on Y. The model has no way to press a key with the other hand, so this would be a new kind of alternative (a hand swap for a few keys).

The rulings are in `rulings.py` as `R13_JY` (reading 1) but excluded through `UNRESOLVED_FAMILIES`. At present the pattern picks J→Y middle on J, index on Y (2.48), and your shipped scorer picks index on J, middle on Y.

## 5. Earlier changes (still in force)

2026-10-28: R-T/T-R and YU both ring-middle / middle-ring; D-A middle-pinky; X→A index→ring < middle→ring < index→middle < any pinky fingering; middle on X + pinky on A out; index+pinky needs ≥ 1.9 key widths; TG middle/index; hand-shift refund (with threshold); "busy default" waiver; K→; and ;→K price identically; your default fingermap enforced on every key (`e_ndgap`); defaults first (rest-key rule, 24 rulings).
2026-10-27: X→W and S→W index→middle (W gate = Q, X or S); ring-X + pinky-A impossible; no IMR on fxw (full-span M/R disallowed); W→D ring/middle, D→W middle→ring preferred; no index on U; S↔Q middle/index both ways; O→K RM, K→; MP, A→E PM, ;→K PM.

## 6. Caveats

- The structure was designed after seeing the rulings (pair cap, M/I full-span step, alternating-hand skip strain, scissor direction, index-straight-up, split non-default penalty, `e_ndgap`, hand shift with threshold, busy default, index/pinky threshold, spread limit). 166/166 in sample is not evidence on its own; the held-out line is.
- The final fit used extra weight on the ladder (`SHIRO_TIER_W`, default 1.5) and a fixed penalty per failing ruling (`SHIRO_FAIL_PEN`, 0.4 in the last searches) because many rulings are near-ties; they steer the search only.
- Legacy rulings were transcribed from your notes and may contain a mistake; round-8 ones were read from `audit_shai.py`.
- Frequencies come from the cfgE-era `results/flow_audit.txt`; `corpus/shai` was in neither zip, so I could not re-anneal.
- An **anchor** (weight 0.04) keeps default-finger pairs where no ruling speaks within 0.25 of your shipped price: 71 of 204 are further away. Pairs involving R, T, Y, X, `.` are excluded.

## 7. Files

| file | what |
|---|---|
| `scorer_geo.py` | drop-in for `scorer2.py`: `get_tables()` → `BIG, TRI, BIG_F, TRI_F` (31 keys + space, 1.0 per press, space 0.6, +12 gate penalty). ~1 s. Needs numpy + scipy. |
| `audit_geo.py` | probes, hard checks and every ruling (`audit_geo_A_all.txt`). |
| `comfort_geo.py`, `gates.py`, `rulings.py` | the model, the gate table, 168 rulings as data with sources. |
| `fitlib.py`, `fit3.py`, `de_polish2.py`, `loo3.py`, `ablate.py`, `review.py` | loss, fit, global polish, hold-out, ablation, frequency-weighted review. |
| `theta_geo_A_all.json` (= `theta3_Aall.json`) | fitted parameters. |
| `review_round8_winner_A_all.md` | comparison with your shipped fingerings. |
| `oldbase.py` / `oldbase.json` | your shipped scorer on the same rulings. |

To use it in `search2` / `anneal_shai`: `from scorer_geo import get_tables, Corpus, SPACE` in place of `scorer2`.
