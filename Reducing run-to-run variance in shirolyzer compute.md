# Reducing run-to-run variance in shirolyzer compute

Analysis of three consecutive `compute` runs · November 2, 2026

## Short answer

Yes, more random restarts will help. But in your outputs the bigger problem is probably that the search is optimizing the wrong number, and more restarts won't fix that. This analysis is based on what I've seen of the code and your logs, not on new runs.

## 1. Fix the score mismatch first

In your three runs, one layout, `ftrng,uahksdlpcwiomevxqbzj.;/y`, comes out with `final` anywhere from 3.4944 to 3.8666, while its `primary` is always 3.8666. The annealer steers by `final` (its incremental running score), but you select by `primary` (the true recomputed score). Where they disagree, the search follows a wrong gradient.

The effect is visible in the logs:

- About half of all starts end in the same `ftrng…` basin, which scores a mediocre 3.8666. It looks great to the annealer (`final` often 3.5–3.7) and poor to the real objective.
- ILS "improvements" don't carry over. In the last run, ILS2 went from 3.6340 to 3.4943, yet its primary is 3.8624.

Three tests in `search.rs` (incremental score equals full score) also fail on the original code. If the incremental score drifts, extra restarts mostly sample more noise. Until those tests pass, tuning restart counts is not trustworthy.

## 2. Then add more restarts, smartly

The project's own tuning report says 25k iterations reach the same optimum as 200k. That means depth can be traded for breadth at about the same cost:

- `--runs 200 --iters 25000` costs roughly what 28 runs × 200k iterations does. This is a hypothesis from the report and should be tested.
- Successive halving is better still: run many cheap anneals, keep the top \~10%, then spend the long anneals only on those.

## 3. Other changes for consistency

- **Stop on rediscovery, not on a fixed count.** Run batches of restarts until the best layout has been found independently by k starts (the new `hits` column tracks this), or until the best primary hasn't improved for N batches. This is the most direct way to make two consecutive runs agree, since it only stops once the answer is confirmed.
- **Make ILS candidates diverse.** ILS takes the top 4 distinct layouts, but "distinct" currently counts a swap of `;,./` as a different layout. The first run's `valid2` is exactly such a sibling. Deduplicate by letter placement (for example, layouts differing in fewer than \~4 letter keys count as the same) and optimize punctuation as a separate final step. Make the number of ILS candidates a flag instead of the hardcoded 4.
- **Measure variance, then pick a budget.** Run the same settings under 20 different seeds and look at the spread of best primary. Raise `--runs` until the spread is small. The three runs here gave 3.5913, 3.6251 and 3.5913, and earlier runs found 3.5207, so the run-to-run range is real.

## Different opinions

- **Some variance may be acceptable.** If the differences between layouts fall within the "valid" tolerance (0.002), they are equivalent for practical purposes. But 3.5207 versus 3.6251 is about 3%, which is probably not equivalent.
- **Cheaper alternatives to restarts:** tune the anneal temperatures (fixed at 0.35 → 0.002), or try parallel tempering. I haven't tested whether these beat plain restarts here.
- **Warm starts are available but off by default.** Starting from the best-known layouts (`--seed-reference`) makes runs agree more, but only because they all begin near the same answer. That hides variance rather than reducing it, so random starts should stay the default.

## Suggested next steps

1. Make the three failing `search.rs` tests pass (find why the incremental score drifts).
2. Implement the rediscovery stopping rule.
3. Deduplicate ILS candidates by letter placement and add an `--ils-candidates` flag.
4. Run the 20-seed variance measurement and pick `--runs` and `--iters` from it.
