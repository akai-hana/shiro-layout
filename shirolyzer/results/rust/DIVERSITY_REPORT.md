# Diversity and near-equal layout report

Date: 2026-10-30. This report answers the specification note: if the generator
returns similarly performing but materially different distributions, record
them so the scoring criteria can be reviewed.

## Selection criteria used by `compute`

A candidate is kept as a valid layout only when:

- `primary - reference_primary <= 0.002` (`--valid-tolerance`)
- `secondary - reference_secondary <= 0.005` (`--secondary-tolerance`)
- it differs from every already-kept layout by at least 4 keystrokes
  (`--min-layout-distance`)
- at most 8 layouts are kept (`--max-valid`)

The reference is the best primary row of the run.

## Observed near-equal layouts

Run: full nine-layout reference file, `--runs 16 --iters 200000 --ils 8
--seed 1234`. The winner is shiro at primary `3.520201897`, secondary
`3.704852567`. The best distinct seed candidates were:

| Primary | Delta vs winner | Key differences | Layout |
|---:|---:|---:|---|
| 3.520201897 | 0.000000 | 0 | `;ntgfjoehkrpmscuaydilbqvwxz,./` |
| 3.521783725 | 0.001582 | 10 | `;dnpfjoitkrchsmuaygelbqvwxz,./` |
| 3.523089721 | 0.002888 | 14 | `;ntckxoehplrgswuaydimqvfbjz,./` |
| 3.523149899 | 0.002948 | 14 | `;nsdfquitkrpchmoaygelbxvwjz,./` |
| 3.523222953 | 0.003021 | 11 | `;nsdvjoitkrpchmuaygelbqfwxz,./` |
| 3.524220888 | 0.004019 | 16 | `;dnpxquitkrchsmoaygelbfvwjz,./` |

Only shiro was written to `valid_layouts`.

## Interpretation

The closest different distribution (`;dnpfjoitkrchsmuaygelbqvwxz,./`) differs
by **ten positions** (well beyond a one- or two-letter swap) and is only
`0.00158` worse on the primary objective, which is inside the primary
tolerance. It was not selected because it came from the random-restart phase
and had no constrained-secondary score: the diversity rule requires the
secondary score to be within `0.005`, and a missing secondary is rejected.

This is exactly the ambiguity the specification asks about:

- The primary objective treats those layouts as effectively tied.
- The analyzer currently has no way to promote a near-equal primary candidate
  that was not already selected for the secondary phase.
- The secondary score and the exact finger/trigram trade-offs are what
  distinguish them, but those values were never computed for this candidate
  inside the run budget.

## Recommendation for future calibration

If these near-equal distributions should be considered seriously, run the
secondary evaluator on every candidate within the primary tolerance (not only
on the top primary row), then compare primary, secondary, and per-finger load
together. A secondary-aware tie-break or a small penalty for large positional
changes would make the selection deterministic and reviewable. No scoring
change was made in this rewrite; the layout is reported here for review as
requested.
