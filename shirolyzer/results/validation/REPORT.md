# Post-rewrite validation

The full compute pipeline ran on the `calibration` branch after the T3 defaults
were applied:

```bash
python shirolyzer/compute.py --name calibration-full --seed 20261029 \
  --out shirolyzer/results/validation/compute-full.txt \
  --out-json shirolyzer/results/validation/compute-full.json
```

- Seed: `20261029`
- References: 8 seeded layouts plus 28 random restarts (36 primary runs)
- Effort: 100000 anneal iterations per run, 8 ILS rounds for each of the top 4
  distinct primary layouts
- Completed: 5.2M anneal iterations, 32 ILS rounds, 4 secondary rounds
- Wall clock: 2205 s

## Winner

The search converged on a single layout:

```
;ntgfjoehkrpmscuaydilbqvwxz,./
```

| metric | value |
|---|---:|
| Primary score | 3.520201897 |
| Secondary score | 3.704852567 |
| Plain trigram score (`analyze.py` / `compare.py`) | 3.394053 |

The top four primary candidates all converged to this layout during ILS and the
secondary phase, so this run selected one clear winner. It was appended to
`shirolyzer/config/references.toml` as `calibration-full`.  The later T5
diversity-aware run found a materially different runner-up with a better
secondary score; see `T5-REPORT.md`.

## Audit

- Enforced rulings: **166 / 166**
- Questionnaire probes and hard checks: **37 / 37**
- `check_baseline.py`: audit 166/166, oldbase 123/168, 9 references

## Reference comparison

All eight pre-rewrite references still match the K12 baseline
(`shirolyzer/config/score_baseline.json`) within 1e-9 for both bigram and
trigram costs, and their ranking is unchanged. The new `calibration-full`
layout improves on the previous best reference:

| layout | trigram score | delta vs pre-rewrite shiro |
|---|---:|---:|
| calibration-full | 3.394053 | -0.149040 |
| shiro | 3.543093 | 0.000000 |
| gallium | 3.588507 | +0.045414 |
| semimak | 3.622450 | +0.079357 |
| sturdy | 3.639963 | +0.096870 |
| canary | 3.668599 | +0.125506 |
| colemak-dh | 3.812333 | +0.269240 |
| colemak | 3.820607 | +0.277514 |
| qwerty | 4.266963 | +0.723870 |

No regressions were observed in the reference scores or in the audit counts.
`check_baseline.py` now verifies every reference present in the baseline while
allowing compute runs to append new references; otherwise the first successful
T4 run would make the baseline fail on the added name.

## Artifacts

- `compute-full.txt`, `compute-full.json`, `compute-full.log` — full search
  output and progress log
- `audit.txt`, `audit.json` — model/ruling audit of the winner run
- `analyze.txt`, `analyze.json` — analyzer report for the winner
- `reference-comparison.txt` — reference ranking with the new layout
