# Shiro layout — provisional Shai-primary result

Status: provisional, 2026-10-26. This is the calibrated Shai-primary
winner from round 8. It is **not** the shipped `cfgE_semiQ` layout; the
known-good cfgE guide is retained in `results/archive/LAYOUT_cfgE.md`.

Detailed report: `findings/REPORT_round8.md`
Audit: `results/round8/audit_shai.txt`
Full per-key analysis: `results/round8/analysis_shai.txt`

## Grid (30 main keys, QWERTY names = physical positions)

```text
        L hand            R hand
top :   ; t s p b        x g o i w
home:   n d h r l        y e c u a
bot :   f k v m z        j q , . /

rest:   A W E F          J I O ;
        pinky ring mid idx | idx mid ring pinky
```

30-char string (`KEY_ORDER`):

```text
;tspbxgoiwndhrlyecuafkvmzjq,./
```

Scores (lower is better):

| metric | score |
|---|---:|
| composite | **3.165795758** |
| primary (Shai) | **3.165796149** |
| secondary (Spanish) | 3.274330031 |
| raw trigram cost | 3.160174631 |
| balance penalty | 0.005621127 |

## What changed and why

This is an unconstrained re-anneal under the calibrated Shai-primary
objective, not an edit of cfgE. The high-frequency letters now sit on the
arched rest keys: `n t s r / e o i a`. The old all-vowels-left arrangement is
not preserved. If that arrangement is a hard design requirement, use a
constrained search instead of this winner.

The model is calibrated to the questionnaire answers about physical-key
motions, not to the old letter placements. The alternate-fingering
prescriptions below are therefore part of the layout: without them the
default-finger SFB rate is `3.38%`; with them it is `0.04%`.

## Fingering guide (actual trigram DP)

Use the alts where they appear. Percentages are share of that key's strokes
in the Shai trigram table.

| key | letter | default finger | actual split |
|---|---|---|---|
| Q | `;` | left pinky | punctuation |
| W | `t` | left ring | R 99.6% / I 0.4% |
| E | `s` | left middle | M 94.0% / R 6.0% |
| R | `p` | left middle | M 50.0% / I 49.6% / R 0.5% |
| T | `b` | left middle | I 56.8% / M 43.2% |
| Y | `x` | right middle | M 82.8% / I 17.2% |
| U | `g` | right middle | M 93.5% / I 6.0% / R 0.4% |
| I | `o` | right middle | M 96.6% / R 3.4% |
| O | `i` | right ring | R 89.3% / M 10.7% |
| P | `w` | right ring | R 100.0% |
| A | `n` | left pinky | P 98.0% / R 2.0% |
| S | `d` | left ring | R 88.4% / M 11.5% / I 0.1% |
| D | `h` | left middle | M 88.6% / I 11.4% |
| F | `r` | left index | I 94.2% / M 5.8% |
| G | `l` | left index | I 100.0% |
| H | `y` | right index | I 100.0% |
| J | `e` | right index | I 97.7% / M 2.3% |
| K | `c` | right middle | M 64.0% / I 19.3% / R 16.7% |
| L | `u` | right ring | R 93.9% / I 5.1% / M 1.0% |
| ; | `a` | right pinky | P 95.8% / R 4.2% |
| Z | `f` | left pinky | P 100.0% |
| X | `k` | left ring | R 94.7% / I 5.1% / M 0.2% |
| C | `v` | left index | I 76.2% / M 23.8% |
| V | `m` | left index | I 91.1% / M 8.9% |
| B | `z` | left index | I 85.0% / M 15.0% |
| N | `j` | right index | I 100.0% |
| M | `q` | right index | I 99.0% / M 1.0% |

The largest alternate users by key are T/`b` (`56.8%`), R/`p` (`50.0%`),
K/`c` (`36.0%`), C/`v` (`23.8%`), Y/`x` (`17.2%`), and B/`z` (`15.0%`).
Typical contexts are in
`results/round8/analysis_shai.txt`; examples:

- T (`b`) index: `but`, `abo`, `bou`, `bee`
- R (`p`) index: `app`, `ppe`, `spe`, `ppo`
- K (`c`) ring: `can`, `act`, `cal`, `ack`; K index: `con`, `com`, `ica`
- C (`v`) middle: `ver`, `vel`, `rov`, `rev`
- L (`u`) index only when the physical O key is in the trigram (it
  holds `i` here): `usi`, `uni`, `uri`, `uti`
- J (`e`) middle: `hey`, `ery`, `yea`, `que`

## Punctuation, space, ISO key

The 30-key optimization leaves the cfgE punctuation plan in place; it was
not re-optimized in this round.

| char | key | note |
|---|---|---|
| `;` | Q key (top-left corner) | left pinky corner |
| `,` | comma key (bottom-right) | natural |
| `.` | period key (bottom-right) | natural |
| `/` | slash key (bottom-right) | natural |
| `'` | ISO-102 key, outside the 30-grid | ANSI fallback: key right of `;` |
| space | right thumb | unchanged |

## Audit summary

- All 12 hard questionnaire probes pass.
- Impossible bigrams: `0`.
- SFB: `6` types / `116,058` tokens (`0.04%`).
- SFS: `514` types / `1,743,852` tokens (`0.73%`); half-SFB `0.17%`.
- Actual alt strokes: `7.81%`; alt-gate violations: `0`.
- Balance: left `17.6 / 26.4 / 28.8 / 27.2%`, right
  `17.0 / 25.3 / 27.8 / 29.9%` (pinky/ring/middle/index).
- Corpus sensitivity: rank `1/16` on Shai, chat_write, code, word, wiki,
  and Norvig root; rank `2/16` on Spanish.

## Main open question: `th`

`t` is on W (left ring) and `h` is on D (left middle). That makes `th` a
mild adjacent ring/middle scissor at physical W->D, cost `0.21`. It occurs
`10,712,957` times and is the largest part of `scissor_MR = 4.24%`.

The model treats adjacent R/M scissors as mild. If that motion feels bad to
you, the next step is to raise the adjacent R/M scissor weight and
re-anneal; the winner would likely move `t` or `h`.

Other known residuals:

- `lm`/`ml` (`101,029` tokens): forced by the G/V index pair.
- `kt`/`tk` (`14,376` tokens): K-alternate versus T-ring collision; the
  optimizer chose the SFB over the full-span scissor.
- `yj`/`jy` (`653` tokens): forced by J having no middle alternate.
- Spanish is rank 2 behind `old_shai_alt1` (`;tncbxgaifsdprlkehuowzvmjqy,./`,
  Spanish `3.180372`). The current layout is the primary winner.

## Files

- `findings/REPORT_round8.md` — full campaign, audit, and open questions.
- `results/round8/audit_shai.txt|json` — hard probes and corpus-wide scan.
- `results/round8/analysis_shai.txt|json` — categories, worst pairs, usage, and
  full per-letter guide.
- `results/round8/anneal_shai_a.txt|json` through
  `results/round8/anneal_shai_d.txt|json` — search campaigns.
- `results/archive/LAYOUT_cfgE.md` — shipped cfgE guide, retained as the
  known-good baseline.
- `results/archive/analysis.json` — legacy cfgE analysis; use
  `results/round8/analysis_shai.json` for the current result.
