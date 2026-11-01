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
   These are currently blocked by geometry reachability; see below.
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
- `C H X` uses `LR LI LP`; `H X` uses `LI LP` (blocked on reachability).
- `Q W W` remains feasible.
- `cargo test --release` passes.
- The change is committed on the branch with a message referencing issue #91.

## Blocked: requested CHX/HX fingerings are unreachable

The current `geometry/ansi.yaml` reach map does not allow the requested fingers
for the physical keys:

| key | reachable fingers | requested |
| --- | --- | --- |
| C | `LM`, `LI`, `RI` | `LR` (unreachable) |
| H | `LR`, `LM`, `LI` (cross), `RM`, `RI` | `LI` (reachable) |
| X | `LR`, `LM`, `LI` | `LP` (unreachable) |

So `{ngram: [C, H, X], fingers: [LR, LI, LP]}` and
`{ngram: [H, X], fingers: [LI, LP]}` cannot build: `Tables::build` reports
`no feasible fingering` because the model marks `C:LR` and `X:LP` as infinite.
Explicit alt rules constrain finger choice but do not override model
reachability.

Options:

1. Use reachable alternatives (for example `C:LM` or `C:LI`, and `X:LR` or
   `X:LM`).
2. Relax reachability for fingers named by an explicit `allow` rule, assigning
   a large finite cost instead of infinity.
3. Keep the requested rules out of the shipped config and use them only in a
   test fixture that stubs reachability.
