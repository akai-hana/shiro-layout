# Alt-finger positional overlap plan

Issue: https://github.com/akai-hana/shiro-layout/issues/91
Branch: `fix/alt-finger-positional-overlap`

## Problem

`Gates::penalty_mask_into` selects the longest matching `allow` rule anywhere
in the typed sequence and drops every shorter rule. A shorter rule that does
not overlap the longer rule is therefore ignored, even though the README says
that a longer rule only overrides shorter rules it overlaps.

Concrete failure with the shipped `config.yaml`:

- `{ngram: [H], fingers: [LI], mode: allow}` is ignored inside physical `C X H`
  because `{ngram: [C, X], fingers: [LI, LM], mode: allow}` is longer.
- `C X H` resolves to `LI LM RI` instead of `LI LM LI`.
- `H X` resolves to `LI LR`, showing that the H rule works when no longer rule
  matches.

## Intended semantics

- Match rules by ordered physical-key sequence.
- A longer rule overrides a shorter rule only when their matched ranges overlap.
- Non-overlapping shorter rules remain active.
- `allow` rules constrain the matched range to the listed fingering.
- `block` rules forbid exactly the listed fingering for the matched range.
- Cross-hand fingers still require an active `allow` rule at that position.
- Repeated definitions of the same ngram remain ordered fallbacks.

## Implementation steps

1. Replace the global `allow_len` scan with an explicit list of rule matches:
   `(start, len, rule_index)`.
2. Mark a match inactive when a strictly longer match overlaps its range.
3. Group active `allow` matches by `(start, len)` and collect active `block`
   matches.
4. Apply block rules positionally; reject tuples that match a blocked range.
5. Apply allow groups positionally; reject tuples outside every active group's
   listed fingering and accumulate fallback penalties.
6. License cross-hand fingers only from active allow groups that cover the
   position.
7. Rework `config.yaml` for repeated-key contexts that become infeasible under
   positional semantics. The known case is `Q W W`: `Q W` forces the first W to
   `LM`, while the `W` unigram block forbids `LM` at the second W.
8. Add the requested concrete rules:
   - `C H X -> LR LI LP`
   - `H X -> LI LP`
   `C:LR` and `X:LP` are outside the geometry reach map, so explicit allow
   rules must also override reachability; see the resolution below.
9. Add gates unit tests for:
   - non-overlapping shorter allow rules still apply;
   - overlapping longer allow rules override shorter rules;
   - cross-hand licensing still requires an explicit active allow rule;
   - the requested `C H X` and `H X` fingerings once reachability is resolved.
10. Run `cargo test --release`, the finger-fingering example, and the relevant
    CLI checks. Record any baseline/audit changes caused by the config rework.

## Risks

- Positional semantics expose contradictions that the old global override hid.
  Repeated keys with unigram block rules are the main source of infeasibility.
- Adding broad repeated-key rules can over-permit fingerings that the old
  `GATES_ALL` translation did not allow. Prefer context-specific rules where
  possible and document any intentional relaxation.
- `config.fixed.yaml` ignores alt rules, so fixed-mode parity should not change.
- Default audit/baseline expectations may change if the rework alters which
  fingerings are reachable. Update expectations only after verifying the
  analyzer output against the intended semantics.

## Acceptance

- `C X H` honors the H unigram allow rule.
- `C H X` uses `LR LI LP`; `H X` uses `LI LP`.
- `Q W W` remains feasible.
- `cargo test --release` passes.
- The change is committed on the branch with a message referencing issue #91.

## Resolution: explicit allow rules override reachability

`C:LR` and `X:LP` are outside the geometry reach map, so the requested rules
could not build: `Tables::build` reported `no feasible fingering`. We took
option 2 from the original list: an active `allow` rule licenses the exact
finger it names even when geometry would mark that key/finger unreachable. The
normal travel and effort weights still price the fingering, so the result is a
finite but expensive cross-body reach rather than a free one.

Implementation:

- `Model::tables_unrestricted` rebuilds the key/pair/trigram tables without the
  reachability infinity.
- `Tables::build` uses the unrestricted tables only for fingerings that are
  both explicitly named by an active `allow` rule and otherwise unreachable.
- `Gates::explicit_fingers` exposes the fingers named by active allow groups so
  the scorer can make that decision per position.

Because `H X` forces `X:LP`, it overlaps existing `X A`, `X C`, and repeated
`X` contexts at the same position. The shipped config now carries longer
context rules that keep those trigrams feasible:

- `H X A -> LI LP {LR,LM,LP}`
- `H X C -> LI LP {LI,LP,LR,LM}`
- `H X X -> LI LP LP`
- `X H X -> LP LI LP`

Reachability relaxation is deliberately global for explicit allow rules, so
previously inert rules that named unreachable fingers (for example `C:LP`,
`Q:LI`, `X:LP`, `S:LP`, `O:RP`, `O:RI`) now take effect. This changes the
user-config scores while the same-hand reference config used by the regression
tests is unchanged. `score_baseline.json` still describes that reference model;
running `check-baseline` against `config.yaml` therefore reports different
reference scores. That mismatch predates this change (it already occurs at
`71dc36f`) and is recorded here rather than folded into the reference baseline.

Verification:

- `cargo test --release`: all unit, baseline, command, and parity tests pass.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `just dev-tools` pass.
- New tests cover the requested `C H X` / `H X` fingerings at the gates and
  scorer level, including the unreachable `C:LR` / `X:LP` states.
