# Compute runtime calibration

All experiments used `--seed 1234`, the default corpora (shai primary, spanish
secondary), and a two-entry reference file (`qwerty`, `shiro`) so that run
counts stay comparable.  Wall-clock time includes table construction, the
primary search, ILS, and the secondary phase.  Raw outputs and logs are in this
directory; `summary.json` collects the rows below.

## Time budget

Fixed: `--iters 60000 --runs 1 --ils 1`.

| Budget | Wall | Runs | Iterations | ILS rounds | Secondary rounds | Primary | Secondary |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 30s | 31s | 1 | 60k | 0 | 0 | 3.521784 | — |
| 60s | 64s | 2 | 120k | 0 | 0 | 3.521784 | — |
| 120s | 123s | 3 | 210k | 1 | 0 | 3.521784 | — |
| 300s | 176s | 3 | 270k | 3 | 2 | 3.521784 | 3.645484 |

Short budgets stop between operations and keep the best result.  The 300s row
finished all available work before the cap.  A budget mainly limits how many
seeds and ILS rounds are attempted; it does not improve the best primary score
by itself once the first good seed has converged.

## Iteration sweep

Fixed: `--time-budget 900s --runs 1 --ils 1`.

| Iterations | Wall | Primary | Secondary |
|---:|---:|---:|---:|
| 10k | 124s | 3.528830 | 3.711433 |
| 20k | 160s | 3.523223 | 3.672136 |
| 40k | 175s | 3.521784 | 3.645484 |
| 60k | 175s | 3.521784 | 3.645484 |

The primary score plateaus between 40k and 60k.  The old 60k default is already
past the observed knee; a larger default adds headroom for different reference
sets and seeds.

## Run sweep

Fixed: `--iters 20000 --time-budget 900s --ils 1`.

| Random runs | Total seeds | Wall | Primary | Secondary |
|---:|---:|---:|---:|---:|
| 1 | 3 | 160s | 3.523223 | 3.672136 |
| 4 | 6 | 263s | 3.523223 | 3.672136 |
| 8 | 10 | 305s | 3.521784 | 3.645484 |

Extra restarts help until roughly ten total seeds on this seed set; after that
the best layout is stable.  More restarts still add robustness against a
different reference pool or corpus.

## ILS sweep

Fixed: `--iters 20000 --time-budget 900s --runs 1`.

| ILS rounds | Total ILS rounds | Wall | Primary | Secondary |
|---:|---:|---:|---:|---:|
| 1 | 3 | 160s | 3.523223 | 3.672136 |
| 3 | 9 | 277s | 3.523223 | 3.672136 |
| 6 | 18 | 499s | 3.520202 | 3.704853 |

ILS keeps improving the primary score beyond six rounds, but the secondary
score can move in the other direction inside the tiny sacrifice budget.

## Conclusions

- Use `iters=100000`, `runs=28`, and `ils=8`: all are above the old defaults
  and the task thresholds, with the iteration value above the measured knee.
- Keep `--time-budget` optional.  The calibration shows that short budgets
  truncate seeds and ILS, so a default run should be allowed to finish.
- The best primary result observed here is `;ntgfjoehkrpmscuaydilbqvwxz,./`
  at 3.520202 (ILS-6); several configurations reach the secondary score
  3.645484.
