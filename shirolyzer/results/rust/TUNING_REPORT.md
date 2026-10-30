# Rust generation-parameter tuning report

Date: 2026-10-30. All runs used the Rust release binary, `--seed 1234`, the
default corpora (shai primary, spanish secondary), and a **qwerty-only**
reference file. The reference file deliberately excludes shiro so the search
has to rediscover it; results are appended to a temporary file, never to the
repository's `references.toml`. Raw logs and JSON are in
`shirolyzer/results/rust/tuning/`.

## Iteration sweep

Fixed `--runs 8 --ils 4`.

| Iterations | Wall | Primary | Secondary | Valid |
|---:|---:|---:|---:|---:|
| 25,000 | 18.0 s | 3.520201897 | 3.704852567 | 1 |
| 50,000 | 21.7 s | 3.520201897 | 3.704852567 | 1 |
| 100,000 | 28.3 s | 3.520201897 | 3.704852567 | 1 |
| 200,000 | 43.3 s | 3.520201897 | 3.704852567 | 1 |
| 400,000 | 73.2 s | 3.520201897 | 3.704852567 | 1 |

The search reaches the shiro layout at 25k and never changes. The iteration
knee is therefore at or below 25k for this corpus/seed; 400k buys nothing.

## Restart sweep

Fixed `--iters 200000 --ils 4`.

| Random runs | Total seeds | Wall | Primary | Secondary |
|---:|---:|---:|---:|---:|
| 4 | 5 | 42.4 s | 3.520201897 | 3.704852567 |
| 8 | 9 | 44.3 s | 3.520201897 | 3.704852567 |
| 16 | 17 | 48.6 s | 3.520201897 | 3.704852567 |
| 28 | 29 | 53.1 s | 3.520201897 | 3.704852567 |

Four random restarts already find shiro. More restarts add robustness but no
quality change on this seed set.

