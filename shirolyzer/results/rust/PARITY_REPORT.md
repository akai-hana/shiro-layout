# Rust simple-port parity report

Date: 2026-10-30. Rust binary: `rust/target/release/shirolyzer` (crate
`shirolyzer` 0.1.0, Rust 1.99.0). Oracle: the frozen Python implementation
`.venv/bin/python -m shirolyzer`.

Commands were run from the repository root. Numeric comparison used absolute
and relative tolerance `1e-9`; table construction was compared at `1e-12`.

## Results

| Check | Command | Result |
|---|---|---|
| Baseline references and audit | `shirolyzer check-baseline` | `baseline OK: audit 166/166, 9 references` |
| Analysis JSON | `analyze ";ntgfjoehkrpmscuaydilbqvwxz,./" --out-json` | Every key, category, trigram pattern, usage row, guide context, and worst-bigram entry matches. The three largest scissor weights differ by one `f64` ULP (relative `~5e-16`) because pair costs are summed in a different order. Primary score `3.3940533330472467` vs Python `3.3940533330472604` (diff `1.4e-14`). |
| ASCII render | `render ";ntgfjoehkrpmscuaydilbqvwxz,./"` | Byte-identical |
| XKB output | `xkb --layout ... --name mine --out ...` | Byte-identical to Python and to `xkb/mine` |
| Reference ranking | `compare` | Byte-identical |
| Audit JSON | `audit --out-json` | Rulings match (`passed=166`, `total=166`), all probe checks and anchors match, and corpus totals serialize as integers like Python. The default `config` is `null`; an explicit model name such as `A:all` is recorded when supplied. |
| Default output paths | `analyze`/`audit`/`compute` without `--out` | Same `analysis_<name>.txt`, `audit.txt`, and `compute_<name>.txt`/`.json` names as Python. |
| Compute structure | `compute --references refs_two.toml --runs 0 --iters 10 --ils 1 --seed 1234` | Phase order, best/selected/valid layout, and all deterministic scores match. Best primary `3.520201897371121` vs Python `3.520201897371135`. |

## Tolerance and RNG

- Table and score parity is within `1e-9` relative (in practice `~1e-14`). Large
  aggregated products such as scissor weights can differ by one `f64` ULP.
- `compute` is compared on structure and on deterministic trajectories.
  Python's `random.Random` and Rust's ChaCha8/StdRNG are different generators,
  so arbitrary random restarts are *quality-comparable*, not
  decision-identical. Runs seeded only by reference layouts produce the same
  layouts and scores.
- Corpus maps are sorted before building evaluator vectors so Rust results are
  independent of hash-map order and thread count.
- The human-readable `audit` text keeps the same rulings and PASS/FAIL results
  but abbreviates some probe annotations; the JSON payload is the compared
  artifact. Default `config` and corpus-count types now match Python.

## Test coverage added for the port

`rust/tests/baseline.rs`, `rust/tests/parity.rs`, and `rust/tests/commands.rs`
assert reference scores, table dimensions and sampled table values, diversity
selection, render, XKB, analysis score, compare, audit, `check-baseline`, and a
short compute run. `rust/src/search.rs` also has a randomized test proving that
incremental swap/cycle deltas equal full rescoring within `1e-12`.

## Verdict

The simple Rust port is functionally stable and numerically equivalent to the
Python oracle for all deterministic behaviour. The only expected difference is
the random-generator family used by exploratory search.
