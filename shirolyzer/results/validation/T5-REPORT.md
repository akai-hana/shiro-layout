# Diversity-aware validation (T5)

T4 selected one primary winner because its top candidates converged to the
same layout.  T5 reran the search with the T4 winner and runner-up as seeds,
more randomization, and diversity-aware clustering/polishing:

```bash
python shirolyzer/compute.py --name t5-diversity --seed 20261030 \
  --references shirolyzer/results/validation/t5-seeds.toml \
  --append-references shirolyzer/config/references.toml \
  --runs 24 --iters 60000 --ils 2 \
  --polish-rounds 3 --polish-candidates 6 \
  --valid-tolerance 0.002 --secondary-tolerance 0.005 \
  --min-layout-distance 4 --max-valid 8 \
  --out shirolyzer/results/validation/t5-polish.txt \
  --out-json shirolyzer/results/validation/t5-polish.json
```

- Seed: `20261030`
- References: 2 seeded layouts plus 24 random restarts (26 primary runs)
- Effort: 60000 anneal iterations per run, 2 ILS rounds on the top 4 distinct
  primary layouts, up to 3 polish passes on the top 6 valid candidates
- Completed: 1.92M anneal iterations, 8 ILS rounds, 3 secondary rounds,
  2 polish passes; wall clock 965 s
- Selection: `multiple_valid`, 2 valid layouts, not truncated

## Valid layouts

| layout | reference | primary | secondary | primary delta | key distance |
|---|---|---:|---:|---:|---:|
| `;ntgfjoehkrpmscuaydilbqvwxz,./` | `calibration-full` | 3.520201897 | 3.704852567 | 0 | 0 |
| `;dnpfjoitkrchsmuaygelbqvwxz,./` | `t5-diversity-2` | 3.521783725 | 3.645483980 | +0.001581828 | 10 |

The two layouts differ in 10 key positions, well above the configured
minimum distance of 4.  The runner-up costs 0.045% more on the primary corpus
(inside the 0.002 window) but scores 0.059369 better on the Spanish secondary
corpus:

```
primary:   ;ntgfjoehkrpmscuaydilbqvwxz,./
secondary: ;dnpfjoitkrchsmuaygelbqvwxz,./
```

This is not a clear win, so both layouts are reported in the text output and
in the JSON `valid_layouts` list.  Both were polished to convergence: each
stopped after one non-improving polish pass, and neither ILS nor polishing
changed their scores.

`selected` remains the T4 winner because the configured primary-sacrifice
budget is 0.00023% (8.1e-06), while the runner-up gives up 0.00158 primary
score.  The `selection_status` field and the valid-layout list make the
alternative explicit; the final choice between them is a human calibration
decision.

## References and checks

- `calibration-full` was already present, so only the runner-up was appended,
  as `t5-diversity-2`.  The reference file now has 10 entries.
- `check_baseline.py`: `baseline OK: audit 166/166, oldbase 123/168, 10 references`
- Full test suite: 34 tests pass.

## Artifacts

- `t5-polish.txt`, `t5-polish.json`, `t5-polish.log` — T5 search output,
  structured results, and progress log
- `t5-seeds.toml` — seed reference file (T4 winner and runner-up)
- `T5-REPORT.md` — this report
