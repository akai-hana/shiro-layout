# Rust optimization summary

Date: 2026-10-30. Epic: `RUST_OPTIMIZATION_TASKLIST.md`. The optimization
branches start from `main` at `9e6e1d7` (takeover completion) and finish on
`opt-verify`. The annotated tag `rust-optimization` marks the final commit.

## Outcome

The Rust generator and analyzer are materially faster without changing the
objective, the calibrated defaults, the corpora, or the audit rulings.

- Generator: the constrained secondary phase now uses dual-delta scoring, ILS
  rows and polish candidates run in parallel with deterministic per-candidate
  RNG streams, key/trigram ids are `u8`, and the cross table is a dense
  move-indexed `Vec`.
- Analyzer: bigram/trigram table rows build in parallel, the report passes run
  concurrently, fingering-guide contexts avoid per-entry strings, and the
  report loops use numeric corpus ids.
- Pruning: audited and rejected. No move-set pruning, new flag, or environment
  hook ships.

## Final numbers

Compared with [`OPT_BASELINE.md`](OPT_BASELINE.md) at `9e6e1d7`:

| Metric | Baseline | Final | Change |
|---|---:|---:|---:|
| Small `compute` | 3.92 s | 1.95 s | 2.01x faster |
| Default `compute` | not measured | 30.64 s | material reduction from the small-run ratio |
| `analyze` median | 0.045 s | 0.0281 s | 1.60x faster |
| `compare` median | 0.031 s | 0.0168 s | 1.85x faster |
| `audit` median | 0.032 s | 0.0211 s | 1.52x faster |
| `check-baseline` median | 0.036 s | 0.0283 s | 1.27x faster |
| Delta-scored quench | 221.3 ms | 165.2 ms | 1.34x faster |
| Single-swap delta | 15,835 ns | 11,639 ns | 1.36x faster |
| Anneal | 61,344 iters/s | 86,293 iters/s | 1.41x faster |

The small `compute` run and the full default run both select
`;ntgfjoehkrpmscuaydilbqvwxz,./` with primary `3.520201897371121` and
secondary `3.7048525673778605`.

## Parity and determinism

- `check-baseline`: audit 166/166 with nine references.
- `analyze`: primary `3.3940533330472467`, within `1.3e-14` of the frozen
  baseline.
- `audit`: probes+checks 37/37, rulings 166/166.
- The randomized incremental-versus-full, constrained dual-delta, and
  delta-versus-full quench tests pass within `1e-12`.
- A fixed-seed run produces identical JSON with `RAYON_NUM_THREADS=1` and
  `RAYON_NUM_THREADS=16`, apart from elapsed time and the input-only append
  path.

## Branches and issues

| Branch | Issue | Scope |
|---|---|---|
| `opt-bench` | #76 | Baseline and profiling |
| `opt-plan` | #77 | Plan and roadmap |
| `opt-generator` | #78-#81 | Generator optimization |
| `opt-analyzer` | #82-#85 | Analyzer optimization |
| `opt-prune` | #86-#87 | Pruning audit and decision |
| `opt-verify` | #88-#89 | Final gates, summary, tag |

Every branch and issue is published. `main` is fast-forwarded to the final
optimization commit.

## Decisions

- **CPU only.** The GPU decision from the rewrite epic is unchanged; the
  optimized CPU search remains the runtime.
- **No pruning.** All 44,850 two-swap cross lists are non-empty, and the
  bounded-subset, phase-skip, and rare-trigram prototypes changed hard-set
  quality. The full move set remains.
- **No objective or default changes.** Scores, references, corpora, and the
  calibrated generation defaults are unchanged.

## Reports

- [`OPT_BASELINE.md`](OPT_BASELINE.md) — frozen pre-optimization evidence
- [`OPT_GENERATOR.md`](OPT_GENERATOR.md) — generator changes and benchmarks
- [`OPT_ANALYZER.md`](OPT_ANALYZER.md) — analyzer changes and benchmarks
- [`OPT_PRUNING.md`](OPT_PRUNING.md) — pruning audit and decision
- [`OPT_VERIFICATION.md`](OPT_VERIFICATION.md) — final gate results

## Rollback

Before the tag, `main` still pointed at `9e6e1d7`. After the tag,
`git checkout rust-optimization^` recovers the pre-optimization tree. No data
or history was deleted.
