# `gwhts` layout remediation plan

Status: **in progress**. Source review:
`research/findings/gwhts_layout_review.md`.

This plan covers every actionable item from the review. It also records the
immediate cleanup of the uncommitted layout work and the stale issue tracker.

## Progress checklist

- [x] Immediate cleanup: commit the computed layout, review, and plan.
- [x] Close stale issues #104, #103, #96, and #93; tracker starts empty.
- [x] W1 — True same-finger metrics with alt-aware reporting.
- [x] W2 — Enforce the Ctrl shortcut constraints.
- [x] W3 — Comparison and reference-score integrity.
- [ ] W4 — Human-factors reporting.
- [ ] W5 — Repeat-aware budget decision and SFS re-anneal.
- [ ] W6 — Final release and tracker cleanup.

## Decisions that constrain the plan

- Flexible, context-dependent fingering is the intended typing model. Travel
  cost, rolls, redirects, and the configured alt rules are the objective.
- Fixed-fingering results are a diagnostic only. They must not become a
  regression gate, and they must not be used to penalise alt fingering.
- Repeats are not same-finger bigrams. A repeated physical key always uses the
  same finger, so counting it as an SFB/SFT/SFS overstates the risk.
- The true same-finger metrics must use the fingers actually selected by the
  flexible DP. Reports should also show the default-fingering equivalent and
  the difference, so the effect of alt fingering is visible without changing
  the score.
- Diagnostics and reports do not change the fitted primary score. A score or
  budget change needs its own gated experiment.
- Every work item gets one issue, one short-lived branch, a tasklist update,
  and commits with `Refs #<issue>` in the body.
- Keep `main` green. Merge with `--no-ff` only after `just check` and the
  relevant regression gates pass.

## Immediate cleanup in this change

The working tree contains the computed-layout adoption plus supporting model
and documentation work. It is grouped into three commits:

| Commit | Scope |
|--------|-------|
| `config: adopt gwhts layout with hard-SFB and redirect updates` | `sfb_impossible` config/model/analyzer, 6-tier redirects, xkb, layout fixtures, references, baseline, README metrics |
| `docs: record gwhts layout review` | `research/findings/gwhts_layout_review.md` |
| `docs: plan gwhts remediation` | this plan |

The existing open issues are stale:

| Issue | Action |
|-------|--------|
| #104 P9 final preference calibration | close as completed; commits `c972e85`, `eb153b6` |
| #103 P8 frequency-weighted finger priority | close as completed; commits `4ae4892`, `987081e`, `cac68cc`, `cf8c99f` |
| #96 P1 preference metrics and sweep harness | close as completed; analysis/preference metrics and sweep tooling are on `main` |
| #93 analyzer phase 2 deferred observability | close as superseded; remaining items 4–8 are folded into W3/W4/W5 below |

After this change, `main` is the only branch and the tracker is empty. New
work-item issues are created only when their branch starts, so the tracker does
not accumulate speculative tickets.

## Git workflow for each work item

```sh
git switch main
git pull --ff-only
gh issue create --title "<issue title>" --body "<scope and acceptance>"
git switch -c <branch>
# implement and commit with Refs #<issue> in the body
just check
gh issue close <issue> --comment "Implemented on <branch> and merged."
git switch main
git merge --no-ff <branch>
git branch -d <branch>
git push origin main
```

Tests, formatting, clippy, and the ignored gates are run before the merge. The
issue stays open until the merge is on `main`.

## W1 — True same-finger metrics with alt-aware reporting

Issue title: `analyzer: report repeat-free, alt-aware same-finger metrics`

Branch: `analyzer/true-same-finger-metrics`

Commits:

1. `analyzer: separate repeated keys from same-finger trigrams`
2. `analyzer: report chosen and default-finger metrics`
3. `tests: pin repeat-free same-finger classification`
4. `docs: document true same-finger metric semantics`

Problem: the current `sfb_tri`, `sft`, `sfb_sft`, and `sfs` counts include
repeated characters. On the current layout `sfb_sft` is exactly the adjacent
repeat share, so the reported "same-finger trigram" number is not a true
same-finger metric.

Definitions:

- **repeat-adjacent:** at least one adjacent pair is the same physical key.
- **repeat-skip:** first and third are the same physical key, middle differs.
- **true SFT:** three distinct keys, all three selected fingers the same.
- **true SFB trigram:** adjacent distinct keys, same selected finger, not a
  true SFT.
- **true SFS:** first and third distinct keys, same selected finger, middle
  finger different.

Implementation tasks:

- Add a key-aware trigram classifier. Keep the finger-only classifier for
  compatibility only if existing callers still need it.
- Emit `sfb_true`, `sft_true`, `sfb_sft_true`, `sfs_true`,
  `repeat_adjacent`, `repeat_skip`, and `repeat_all`.
- Keep the legacy `sfb_tri`/`sft`/`sfb_sft`/`sfs` fields for one release and
  mark them `legacy_*` in the text report. The legacy fields retain their
  current budget meaning; the new fields are the honest diagnostics.
- Compute the new metrics twice: once with the chosen DP fingers (flexible)
  and once with the default fingermap. Report the default values as
  `*_default` and the delta. This is the alt-fingering factor requested in the
  review.
- Add the same fields to `worst` and the JSON payload.
- Add synthetic tests covering `aaa`, `aba`, `abb`, `abc` with same/different
  fingers, and a case where the default fingering has an SFB but the chosen
  alt fingering does not.
- Update the README taxonomy and the analysis command examples.

Acceptance:

- `sfb_true = 0`, `sft_true = 0`, and `sfb_sft_true = 0` for the current
  `gwhts` layout under flexible fingering.
- `repeat_adjacent` equals the old `sfb_sft` within corpus rounding.
- The default-fingering report is present and differs from the chosen report
  where alt rules apply.
- The primary score, budget selection, and existing fixtures do not change.
- `just check` passes.

Out of scope for W1: changing the fitted score or the sacrifice budget. The
budget split is W5.

## W2 — Enforce the Ctrl shortcut constraints

Issue title: `config: enforce Ctrl shortcut hand and finger constraints`

Branch: `config/shortcut-constraints`

Commits:

1. `config: restrict t and y to the left hand`
2. `analyzer: report Ctrl shortcut collisions`
3. `config: adopt the shortcut-constrained layout`
4. `docs: record shortcut constraint results`

Problem: `tomorrow.txt` requires `z,x,c,v,t,y` on the left hand. The config
only restricts `z,x,c,v`, and the computed layout puts `y` on the right hand.
`c` is on the left pinky, so Ctrl+C is same-finger with left Ctrl.

Tasks:

- Update `hand_restrictions.left` to `[z, x, c, v, t, y]`.
- Add a report-only shortcut section to `analyze`: for each of
  `z,x,c,v,t,y`, show the physical key, hand, chosen/default finger, and
  whether left-Ctrl + the key is same-hand and same-finger.
- Run a constrained compute with the new restriction. Compare both corpora
  and the full report before adopting.
- If the constrained layout still leaves `c` on the left pinky, add an
  optional finger-aware shortcut rule rather than forcing a swap. The rule
  should be explicit and report-only until a layout is generated with it.
- Update the xkb map, fixtures, references, README, and review metrics if the
  constrained layout is adopted.
- Keep the fixed-fingering diagnostic unchanged.

Acceptance:

- All six shortcut letters are on the left hand.
- Ctrl+C is no longer same-finger, or the collision is explicitly accepted and
  documented.
- The adopted layout is generated by `compute`, not by a hand-forced swap.
- `just check` passes; the fixed-fingering diagnostic is not a gate.

## W3 — Comparison and reference-score integrity

Issue title: `analyzer: make comparisons and stored reference scores config-aware`

Branch: `analyzer/comparison-integrity`

Commits:

1. `analyzer: warn when hard-SFB scoring is used for comparisons`
2. `tests: pin stored shiro score to the shipped config`
3. `docs: document comparison configurations`

Problem: `compare` under `sfb_impossible` gives enormous reference scores, and
stored scores drifted from the shipped config. The current cleanup refreshes
the `shiro` entry, but nothing prevents the next drift.

Tasks:

- Detect `sfb_impossible` in `compare` and print a clear warning with the
  recommended permissive/fixed command.
- Add a `score_config` field next to stored reference scores (values
  `shipped`, `reference`, or `permissive`) and validate it when loading.
- Add a test that the stored `shiro` score matches `analyze` under the
  shipped config, or fail with a regeneration instruction.
- Document which config each stored score belongs to.
- Do not change the fitted score or reference layouts.

Acceptance:

- `compare` cannot silently present hard-SFB reference scores as comparable.
- Stored `shiro` scores stay in sync with the shipped config.
- Existing command, baseline, and parity tests pass.

## W4 — Human-factors reporting

Issue title: `analyzer: report cross-hand, punctuation, and shift-layer factors`

Branch: `analyzer/human-factors-report`

Commits:

1. `analyzer: summarize cross-hand alt usage`
2. `analyzer: report punctuation and shift-layer coverage`
3. `tests: pin human-factors reports`
4. `docs: document human-factors diagnostics`

Problem: the report identifies cross-hand `m`/`o`, dead-key punctuation, the
unmodeled apostrophe, and the unscored shift layer. These are human-factors
risks, not scoring bugs.

Tasks:

- Add a summary of cross-hand alt usage: total share, per-letter share, and
  top cross-hand contexts. Use the chosen fingers; show the default-finger
  equivalent for comparison.
- Add a punctuation/shift report: dead keys, apostrophe key, shifted symbol
  for each moved punctuation key, and a short list of common programming/prose
  bigrams that cross those keys.
- Keep everything report-only. Do not add a cross-hand penalty or a
  punctuation score.
- Add fixture tests with a synthetic layout and config.
- Document the reports and the explicit non-goal of penalising alt fingering.

Acceptance:

- The current layout reports `m` at ~100% cross-hand LI and `o` at ~24% LI.
- The punctuation report lists the dead keys and the apostrophe key.
- The primary score is unchanged.
- `just check` passes.

## W5 — Repeat-aware budget decision and SFS re-anneal

Issue title: `experiment: repeat-aware budget and repeat-free SFS re-anneal`

Branch: `experiment/repeat-aware-sfs`

Commits:

1. `experiment: add configurable repeat budget weight`
2. `analysis: record repeat-aware re-anneal results`
3. `docs: decide whether to adopt the repeat-aware optimum`

Problem: the sacrifice budget counts repeats as SFB/SFT, while the community
treats repeats separately. The review also recommends a repeat-free
SFS-weighted search.

Tasks:

- Add a configurable `repeat_adjacent` budget term. Preserve the current
  effective weight as the default so the score does not change by accident.
- Add a repeat-free SFS term as an experimental objective.
- Run `compute` on both corpora with the experiment config and at least the
  standard seed set used by the calibration harness.
- Compare against the current layout on primary/secondary score, true
  SFB/SFT/SFS, repeats, redirects, rolls, scissors, and finger loads.
- Adopt only if the result wins on the agreed priorities without a
  higher-priority regression. Otherwise record the negative result and close
  the issue.
- Never use fixed-fingering score as the deciding criterion.

Acceptance:

- The experiment is reproducible from a committed config and seed list.
- The decision and raw metrics are recorded in `research/findings/`.
- If adopted, the config, layout, references, and regression fixtures are
  updated together.
- If rejected, no scoring path changes.

## W6 — Final release and tracker cleanup

Issue title: `release: finalize gwhts remediation and refresh locked metrics`

Branch: `release/gwhts-remediation`

Commits:

1. `tests: lock final remediation regressions`
2. `docs: finalize remediation results`
3. `config: adopt final remediation layout and scores`

Tasks:

- Run the full verification suite, including the ignored compute test and
  `just dev-tools`.
- Refresh README, references, baselines, and the review metrics under the
  final config.
- Confirm `main` is the only local and remote branch.
- Confirm no stale work-item issues remain open; close each with its merge
  commit.
- Record the final accepted metrics and any rejected experiments.

Acceptance:

- `just check`, ignored tests, and `just dev-tools` pass.
- `main` is green and is the only branch.
- The issue tracker contains no stale open issues.
- The README numbers match `analyze` under the shipped config.

## Dependency order

```text
cleanup
└── W1 true metrics
    ├── W2 shortcut constraints
    ├── W3 comparison integrity
    └── W4 human-factors reports
        └── W5 repeat-aware experiment
            └── W6 release
```

W1 must land before W2 because W2 changes the layout and the final evaluation
must use honest same-finger metrics. W3, W4, and W5 can proceed after W1; W5
depends on W1's repeat-free diagnostics.

## Verification gates for every work item

```sh
just build
just test
just check
cargo test --manifest-path shirolyzer/Cargo.toml --release -- --ignored
just dev-tools
```

Additional gates:

- no primary-score change for observability-only items;
- no fixed-fingering gate or alt-finger penalty introduced;
- layout-changing items must come from a constrained `compute` run;
- every commit references its issue.
