# Repeat-aware budget and repeat-free SFS experiment

Date: 2026-11-09
Issue: #109
Config: `shirolyzer/config/experiments/repeat_aware.yaml`
Seeds: `0 1 2` (`shirolyzer/config/experiments/repeat_aware.seeds`)
Budget: default compute budget (28 runs, 200,000 iterations, 8 ILS rounds, unlimited polish)
Command: `cargo run --release -- compute --config config/experiments/repeat_aware.yaml --seed N`

Raw JSON: [`repeat-aware/sfs2-seed-0.json`](repeat-aware/sfs2-seed-0.json), `sfs2-seed-1.json`, `sfs2-seed-2.json`.
The budget-only control, run before the SFS multiplier was added, is in `repeat-aware/budget-only-seed-{0,1,2}.json`.

## Search result

The budget split alone selected the shipped layout for all three seeds. After adding `general.sfs_scale: 2.0`, all three seeds selected:

```text
b t h c r | m d i s j
n p l y ; | e o u g a
v k , z x | q . w f /
```

That is `bthcrmdisjnply;eougavk,zxq.wf/`. It differs from the shipped layout `bthcrmdisjnply;eougavkxz,q.wf/` only by swapping `x` and comma across physical `C` and `B`.

| Seed | Compute primary | Compute secondary | Elapsed |
|-----:|----------------:|------------------:|--------:|
| 0 | `4.38033644197296` | `4.607849144844175` | `209.0s` |
| 1 | `4.38033644197296` | `4.607849144844175` | `269.4s` |
| 2 | `4.38033644197296` | `4.607849144844175` | `259.3s` |

## Comparison under the shipped config

Scores and true metrics from `analyze` under `shirolyzer/config/config.yaml`:

| Metric | Shipped layout | SFS-weighted layout | Delta |
|--------|---------------:|--------------------:|------:|
| Primary score | `4.264577544912479` | `4.264583902389809` | `+0.000006357477330` |
| Secondary score | `4.464992637593545` | `4.461978112843553` | `-0.003014524749992` |
| `sfb_true` | `0.000%` | `0.000%` | `0` |
| `sft_true` | `0.000%` | `0.000%` | `0` |
| `sfs_true` | `2.3063%` | `2.2847%` | `-0.0217 pp` |
| `repeat_adjacent` | `6.6729%` | `6.6729%` | `0` |
| `repeat_skip` | `3.3951%` | `3.3951%` | `0` |
| Redirects | `3.9562%` | `3.9678%` | `+0.0116 pp` |
| Rolls | `47.2809%` | `47.2845%` | `+0.0035 pp` |
| Alternation | `38.9327%` | `38.9133%` | `-0.0194 pp` |
| Mild scissor cost | `0.3294` | `0.3294` | `0` |

Finger loads:

| Finger | Shipped | SFS-weighted |
|--------|--------:|-------------:|
| LP | `7.9992` | `7.9995` |
| LR | `13.5699` | `13.6093` |
| LM | `13.6045` | `13.5783` |
| LI | `13.9351` | `13.9280` |
| RP | `7.8677` | `7.8677` |
| RR | `10.5340` | `10.5340` |
| RM | `14.7854` | `14.7842` |
| RI | `17.7041` | `17.6989` |
| Left hand | `49.1087` | `49.1152` |
| Right hand | `50.8913` | `50.8848` |

The score-sacrifice allowance is `0.00023%` of the best primary score, or about `9.81e-6`. The SFS-weighted primary regression (`6.36e-6`) is inside that allowance, and the secondary score improves. The SFS gain is only `0.0217` percentage points, repeats are unchanged, and the layout change is the marginal `x`/comma swap.

Under the experiment config itself, the SFS-weighted layout is the better search result:

| Metric | Shipped layout | SFS-weighted layout |
|--------|---------------:|--------------------:|
| Primary score | `4.337076804091418` | `4.336654142239611` |
| Secondary score | `4.558215208480613` | `4.554398885830656` |
| `sfs_true` | `0.2645%` | `0.2644%` |

## Decision

**Rejected.** The shipped layout and default scoring path stay unchanged:

- Primary score is the first priority, and the SFS-weighted layout is worse under the shipped config (`+6.36e-6`) even though the regression is inside the sacrifice allowance.
- The true SFS improvement is small, repeats are unchanged, and the only placement change is the `x`/comma swap.
- Most of the experimental SFS gain comes from the doubled SFS multiplier changing chosen fingers (`sfs_true` falls from `2.306%` to `0.264%` under the experiment config), not from a clearly better letter arrangement.

The repeat-aware budget and `general.sfs_scale` remain opt-in. The shipped config keeps `sfs_scale: 1.0` and the legacy combined SFB/SFT budget; no references or regression fixtures were changed. The experiment config, seed list, and raw JSON are retained so the negative result is reproducible.

## Validation caveat

This experiment only tests the questionnaire-calibrated model. It is not evidence from human typing data, and it does not establish global optimality.
