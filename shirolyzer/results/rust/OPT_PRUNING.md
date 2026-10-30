# Rust pruning audit and decision

Date: 2026-10-30. Tasks: OPT.T3.1 and OPT.T3.2. This report records the
aggressive-pruning audit, the temporary prototypes, their quality and speed
results, and the shipped decision.

## Environment

| Item | Value |
|---|---|
| Host | AMD Ryzen 9 6900HX, 8 cores / 16 threads, 30 GiB RAM |
| OS | Chimera Linux, kernel 7.2.2-0-generic |
| Rust | rustc 1.99.0, release profile (`lto = "thin"`, `codegen-units = 1`) |
| Base commit | `9749eb8` (`opt-analyzer`, OPT.T2.4) |
| Binary | `shirolyzer/target/release/shirolyzer` |

## Method

Pruning candidates were audited from `search.rs`. The movable search uses the
full pair set (325), the disjoint two-swap set (44,850), and the three-cycle
set (2,600). Prototypes were added temporarily behind private environment
hooks (`SHIRO_PRUNE` for phase or subset pruning, `SHIRO_MIN_COUNT` for rare
trigram pruning), measured with the release binary, and then removed. The
committed binary contains no prototype hooks or new flags.

Two benchmark shapes were used:

1. The tasklist small run: `--runs 0 --iters 10 --ils 1 --seed 1234
   --max-valid 2 --polish-rounds 0` with the shipped references.
2. A harder qwerty-only reference set, so the search must rediscover shiro:
   `--references qwerty.toml --seed 1234 --max-valid 2 --polish-rounds 0`
   with an 8-run/25k-iteration/ILS-2 case and a 4-run/2k-iteration case.

## Candidate audit

| Candidate | Finding | Result |
|---|---|---|
| Empty or short cross lists | 44,850 disjoint two-swap moves; 0 empty lists; minimum 195 entries, mean 505.07, maximum 576 | Nothing safe to skip |
| Dominated moves | Two-swap moves are already unique and unordered; overlapping pairs are excluded at construction | No duplicate or exact dominated move found |
| Repeated no-op deltas | The two-pair loop already caches each single-pair delta for the current accepted state and clears the cache on acceptance | Already optimal; no duplicate work to remove |
| Rare-trigram pruning | Threshold-filtering the cross lists was prototyped at counts 10, 100, 1,000, and 10,000 | Changes the incremental objective; reject |
| Bounded move subsets | A 25% two-swap subset and a one-pass two-swap cap were prototyped | Changes the search path and hard-set quality; reject |
| Early quench exits | Skipping the two-pair phase, the three-cycle phase, or the pair phase was prototyped | Changes results or does not save time; reject |

## Prototype results

### Small tasklist run

The prototype binary was run three times per mode. Wall clock is the median of
three external runs; the selected layout and both scores were identical in
every mode.

| Mode | Wall | Selected | Primary | Secondary |
|---|---:|---|---:|---:|
| Full move set | 1.80 s | `;ntgfjoehkrpmscuaydilbqvwxz,./` | 3.520201897371 | 3.704852567378 |
| Skip two-pair phase | 0.62 s | `;ntgfjoehkrpmscuaydilbqvwxz,./` | 3.520201897371 | 3.704852567378 |
| One-pass two-pair cap | 1.08 s | `;ntgfjoehkrpmscuaydilbqvwxz,./` | 3.520201897371 | 3.704852567378 |
| 25% two-pair subset | 0.90 s | `;ntgfjoehkrpmscuaydilbqvwxz,./` | 3.520201897371 | 3.704852567378 |
| Skip three-cycle phase | 1.52 s | `;ntgfjoehkrpmscuaydilbqvwxz,./` | 3.520201897371 | 3.704852567378 |
| Skip pair phase | 1.84 s | `;ntgfjoehkrpmscuaydilbqvwxz,./` | 3.520201897371 | 3.704852567378 |

This run starts from the shipped references and does not discriminate the
search paths, so the hard qwerty-only runs below are the deciding evidence.

### Hard qwerty-only runs

Mid case: 8 starts, 25,000 iterations, ILS 2, seed 1234.

| Mode | Wall | Best layout | Primary | Secondary |
|---|---:|---|---:|---:|
| Full move set | 3.21 s | `;dnpfjoitkrchsmuaygelbqvwxz,./` | 3.521783724808 | 3.645483980184 |
| Skip two-pair phase | 1.52 s | `;ntgfjoehkrpmscuaydilbqvwxz,./` | 3.520201897371 | 3.704852567378 |
| One-pass two-pair cap | 2.41 s | `;dnpfjoitkrchsmuaygelbqvwxz,./` | 3.521783724808 | 3.645483980184 |
| 25% two-pair subset | 1.89 s | `;dnpfjoitkrchsmuaygelbqvwxz,./` | 3.521783724808 | 3.645483980184 |
| Skip three-cycle phase | 3.06 s | `;dnpfjoitkrchsmuaygelbqvwxz,./` | 3.521783724808 | 3.645483980184 |

Low case: 4 starts, 2,000 iterations, ILS 0, seed 1234.

| Mode | Wall | Best layout | Primary | Secondary |
|---|---:|---|---:|---:|
| Full move set | 2.22 s | `;lnhwquitksrpdcoaygevxbmfjz,./` | 3.527058954663 | 3.665385099704 |
| Skip two-pair phase | 0.63 s | `;stpbjoehkrcgnmuaydilqfvwxz,./` | 3.528694384144 | 3.714857698099 |
| One-pass two-pair cap | 1.18 s | `;rtcfxoehplndswuaybimqgvkjz,./` | 3.524591194929 | 3.725410997060 |
| 25% two-pair subset | 1.05 s | `;ntgvjoerkdlpscuayhimqfbwxz,./` | 3.533374168823 | 3.635238750632 |
| Skip three-cycle phase | 1.77 s | `;stlbjoehkrcmnpuaydiwgqfvxz,./` | 3.534275943798 | 3.707198036313 |

The variants change the selected layout and can be better or worse. That is
exactly the risk the tasklist forbids: the objective and search path are no
longer fixed.

### Rare-trigram thresholds

Cross-list entries were retained only when the trigram count reached the
threshold. The table shows external wall clock in seconds; the selected
layout and scores were unchanged in each column for these runs.

| Minimum count | Small run | Hard mid | Hard low |
|---:|---:|---:|---:|
| 0 (full) | 1.71 | 3.28 | 2.16 |
| 10 | 1.73 | 3.19 | 2.25 |
| 100 | 1.71 | 3.19 | 2.35 |
| 1,000 | 1.48 | 2.94 | 1.94 |
| 10,000 | 1.20 | 2.42 | 1.57 |

A useful speedup only appears at a threshold near the 90th percentile of
trigram counts, where most cross terms are dropped. That is not rare-trigram
pruning; it is a different objective. It would also fail the randomized
incremental-versus-full parity test because omitted cross terms make the
delta score disagree with the full score.

## Decision

No pruning is shipped.

- The default search keeps the full pair, disjoint two-swap, and three-cycle
  move sets.
- No new flag, environment variable, or default is added.
- The prototype hooks were removed before the final gates.
- The gate for this decision is the new test
  `search::tests::delta_quench_matches_full_quench`, which runs the delta
  quench and the full-score `quench_generic` from the same layout with the same
  RNG seed and requires the same permutation and score within `1e-12`. The
  existing randomized incremental-versus-full test and the baseline/parity
  tests remain in place.

Any future pruning must keep the delta quench path decision-identical and
preserve the selected layout and scores on the benchmark seeds. None of the
prototypes above did both.
