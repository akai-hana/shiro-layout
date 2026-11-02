# Analyzer deferred improvements — phase 2

Status: **in progress**. Phase 1 (SFT/SFB split and budget fix, #92) is
merged. Improvement 3 (corpus provenance, #94) is complete and merged into
`analyzer/redirect-tiers`. The remaining deferred work is split so each item
can be reviewed, tested, and reverted independently.

Tracking issue: <https://github.com/akai-hana/shiro-layout/issues/93>
Implementation issue for the first item:
<https://github.com/akai-hana/shiro-layout/issues/94>

## Guiding decisions

- Keep Shiro's trigram-DP comfort model as the scoring source of truth.
  Oxeylyzer's integer weighted sum, full taxonomy, geometry, and skipgram
  decay are not imported.
- Diagnostics and provenance are observability. The primary fitted score and
  the shipped baselines must not change unless a separately gated experiment
  proves a score change is required.
- Every item gets a short-lived branch. Merge it back with `--no-ff` only
  after `just check` and the relevant regression gates pass.
- Update this tasklist in the same commit that completes an item, so the
  checkboxes always describe the branch state.
- Commit convention: `analyzer: <imperative summary>` with
  `Refs #<issue>` in the body.

## Branch and base strategy

Phase 2 is stacked on the pending redirect-tier work:

```text
main
└── analyzer/redirect-tiers        (complete, pending merge)
    └── analyzer/corpus-provenance (this branch, improvement 3)
```

`analyzer/corpus-provenance` is the current integration base for phase 2.
If `analyzer/redirect-tiers` merges to `main` first, rebase this branch onto
`main` before merging. Later improvements branch from the phase-2 base after
the previous item lands.

| # | Improvement | Branch | Issue | Main commits |
|---|-------------|--------|-------|--------------|
| 3 | Corpus provenance | `analyzer/corpus-provenance` | #94 | `docs: plan deferred analyzer improvements`; `analyzer: record corpus provenance in reports` |
| 4 | External-comparison mapping | `analyzer/comparison-mapping` | TBD | `analyzer: add external comparison mapping` |
| 5 | Score decomposition | `analyzer/score-decomposition` | TBD | `analyzer: expose score decomposition` |
| 6 | Naming and semantics cleanup | `analyzer/naming-semantics` | TBD | `analyzer: align redirect and SFB naming` |
| 7 | Geometry known-answer tests | `analyzer/geometry-tests` | TBD | `analyzer: add geometry known-answer tests` |
| 8 | Report-only extras | `analyzer/report-extras` | TBD | `analyzer: add optional skipgram and shift reports` |

## Improvement 3 — Corpus provenance

Problem: `analyze` records corpus labels, but not the parser rule, the exact
source bytes, or the normalization used. Two runs with the same label can
therefore produce different scores with no audit trail.

Scope:

- Add a shared SHA-1 helper (reuse the existing implementation from `xkb`).
- Add a `provenance` module that describes one resolved corpus path:
  - kind: `parsed`, `raw`, or `external-json`;
  - resolved path and configured weight;
  - per-file path, size, and SHA-1, plus a combined corpus hash;
  - parser rule id and human-readable rule;
  - normalization description;
  - entry counts and weighted totals.
- Include `corpus_provenance.primary` and `.secondary` in the `analyze`
  JSON payload and print a short `== corpus provenance ==` text section.
- Allow an external n-gram JSON (`--corpus-json`) for `compare` only. The
  external file is a comparison input; it never changes the native parser or
  the generator's score.
- Add a network-free snapshot test with fixed synthetic files and a fixed
  expected hash.

Provenance schema (per corpus):

```json
{
  "label": "shai",
  "weight": 100.0,
  "kind": "parsed",
  "path": "/.../corpora/shai/parsed",
  "combined_sha1": "5f023eb006ba858203e81c9866698eaf390a7a06",
  "parser": "parsed-json-v1",
  "parser_rule": "read letter/bigram/trigram JSON maps; keep single-char, 2-char, and 3-char keys",
  "normalization": "none (pre-parsed counts)",
  "files": [{"name": "letter_freq.json", "bytes": 377, "sha1": "…"}],
  "totals": {
    "letters": 418381828.0,
    "bigrams": 327253737.0,
    "trigrams": 240524895.0,
    "letter_entries": 26,
    "bigram_entries": 676,
    "trigram_entries": 15437
  }
}
```

Tasklist:

- [x] Create the phase-2 tracking issue and the corpus-provenance issue.
- [x] Create `analyzer/corpus-provenance` from `analyzer/redirect-tiers`.
- [x] Add this plan and tasklist.
- [x] Extract a shared `sha1_hex` helper and keep `xkb` behavior identical.
- [x] Add the provenance descriptor and unit tests.
- [x] Emit provenance in the `analyze` text and JSON output.
- [x] Add the `--corpus-json` comparison path (optional, comparison only).
- [x] Add the offline provenance snapshot test.
- [x] Document the fields and the external-comparison boundary.
- [x] Run `just build`, `just test`, `just check`, ignored tests, and
      `just dev-tools`.
- [x] Confirm this branch is the phase-2 integration base (no intermediate
      merge needed).
- [x] Push the branch, merge it into `analyzer/redirect-tiers` with
      `--no-ff`, and close #94 (`5ad8d49`).

## Improvement 4 — External-comparison mapping and warnings

- Map Shiro categories to community names (`Sfb`, `BadSfb`, `Sft`, `SFS`, …).
- Warn when a comparison mixes bigram and trigram SFB, fixed and flexible
  fingering, or different corpora/denominators.
- Keep the mapping report-only; do not feed it into the score.

Branch: `analyzer/comparison-mapping`.

## Improvement 5 — Decompose Shiro's score

- Expose per-term contributions (pair SFB, scissor, redirect, roll, repeat,
  balance).
- Add the decomposition to `analyze --out-json` and a concise text table.
- Regression-test that the contributions reconstruct the primary score.

Branch: `analyzer/score-decomposition`.

## Improvement 6 — Naming and semantics cleanup

- Align `W_REDIRECT_WEAK` (model: pinky involved) with `redirect_weak`
  (analysis: no index), or rename both to one documented concept.
- Audit every `sfb_tri`/`repeat` use after the SFT split and rename ambiguous
  identifiers.
- Document the final taxonomy in one place and link it from the README.

Branch: `analyzer/naming-semantics`.

## Improvement 7 — Geometry known-answer tests and debug dump

- Add known-answer tests for physical pair distances on the shipped board.
- Add a tests-only debug dump for geometry and reachability.
- Compare Shiro distances directly instead of porting Oxeylyzer geometry.

Branch: `analyzer/geometry-tests`.

## Improvement 8 — Optional report-only extras

- Add a skipgram/SFS distance ladder as a report-only metric.
- Add a shift/punctuation-layer audit.
- Add either metric to the score only after a controlled experiment shows it
  changes Shiro's own optimum.

Branch: `analyzer/report-extras`.

## Verification gates

- [ ] `just build`
- [ ] `just test`
- [ ] `just check`
- [ ] `cargo test --manifest-path shirolyzer/Cargo.toml --release -- --ignored`
- [ ] `just dev-tools`
- [ ] Reference-config audit and baseline tests pass; no intentional score
      change for observability-only work.

## Acceptance criteria for phase 2

- Every reported score can be traced to a corpus label, exact source bytes,
  parser rule, and normalization rule.
- External comparison inputs are opt-in and cannot silently change generation
  or the shipped baseline.
- Each improvement lands on its own branch with its own issue, tasklist
  update, and verification record.
- Deferred items remain independently revertable without reverting unrelated
  analyzer work.
