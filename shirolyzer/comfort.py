"""
comfort.py — Calibrated ANSI + user-preference comfort model.

This replaces the borrowed integer-grid / community-prior model with one built
from (a) real ANSI-staggered geometry and (b) the user's directly-stated
preferences. Every constant here traces to something the user said; see the
"USER:" comments.

Two things the old model got wrong that this fixes:
  1. Distance/scissors/travel are computed in ANSI key-center space (ansi.py),
     not a clean integer grid. This creates genuine left/right asymmetry for
     same-column pairs (ED vs IK), which the user explicitly flagged.
  2. The M<->I roll comfort is no longer a flat "-0.35 if inward". It is a
     per-row-pair preference ladder the user dictated, and it is LEFT-hand
     calibrated (their stated ladder is for the left M/I zone; the right hand
     uses the ANSI-corrected mirror, which is NOT identical because of stagger).
"""
import ansi
from fingers import KEYS

# ---------------------------------------------------------------------------
# USER: fingers are all equal; even distribution is a plus, no per-finger
#       preference. So no STRENGTH-based effort bias and no IDEAL_SHARE tilt.
#       Effort is pure travel from rest (posture), not finger strength.
# ---------------------------------------------------------------------------
# Travel costs (key widths from rest). Downward/lateral cost more than up.
_TRAVEL_DOWN = 0.70
_TRAVEL_UP = 0.40
_TRAVEL_LAT_PER_WIDTH = 1.00

# USER (flow-vs-travel): "flow and comfort matter more ... moving my fingers
# maybe a couple decimals more or less [is trivial] ... travel should be a
# secondary concern". So only the DISPLACEMENT part of travel competes with
# n-gram costs, and it competes at decimal magnitude (x0.2). The +1.0
# per-press baseline is layout-invariant and left alone for readable scores.
TRAVEL_SCALE = 0.20

# Rest keys (arched home AWEF-JIO;). USER confirmed this is deliberate.
REST = {
    ("L", "P"): "A", ("L", "R"): "W", ("L", "M"): "E", ("L", "I"): "F",
    ("R", "I"): "J", ("R", "M"): "I", ("R", "R"): "O", ("R", "P"): ";",
}

# Finger natural column (ANSI x) used when a finger has no rest on this hand.
_NATURAL_CX = {
    "L": {"P": ansi.cx("A"), "R": ansi.cx("S"), "M": ansi.cx("D"), "I": ansi.cx("F")},
    "R": {"I": ansi.cx("J"), "M": ansi.cx("K"), "R": ansi.cx("L"), "P": ansi.cx(";")},
}


def travel_cost(key, finger, hand):
    """Posture cost of pressing `key` with `finger` (0 at rest).

    USER: no finger-strength weighting. Cost is geometric displacement only.
    """
    rest = REST.get((hand, finger))
    if rest is None:
        rdx = ansi.cx(key) - _NATURAL_CX[hand][finger]
        rdy = ansi.cy(key) - 1.5  # home-row center y
    else:
        rdx = ansi.cx(key) - ansi.cx(rest)
        rdy = ansi.cy(key) - ansi.cy(rest)
    down = max(rdy, 0.0) * _TRAVEL_DOWN
    up = max(-rdy, 0.0) * _TRAVEL_UP
    lat = abs(rdx) * _TRAVEL_LAT_PER_WIDTH
    return 1.0 + TRAVEL_SCALE * (down + up + lat)


# ---------------------------------------------------------------------------
# USER: same-key repeats (ll, tt, ee) are unavoidable, should NOT be pooled
#       with genuine inefficiencies (SFB). Industry practice confirms: cmini
#       excludes them from all categories, genkey keeps a standalone
#       SameKeyKPS, keycraft reports doubles as their own table, and SFB code
#       structurally skips i==j. So: a SMALL separate penalty, not SFB weight.
# ---------------------------------------------------------------------------
W_REPEAT = 0.6  # USER: mild, separate class. (SFB same-row is 5.0 by contrast.)

# ---------------------------------------------------------------------------
# USER: scissor direction preference MATCHES the anatomical model —
#       shorter-finger-up / longer-down is bad; reverse is free. Their middle
#       is effectively ~1 key taller than index (why AWEF-JIO; rest), so
#       middle-above-index is the natural stance. Confirmed for IM; treat MR/RP
#       with the same shorter-up rule.
#       BUT: cost magnitude now scales with TRUE ANSI distance, and left/right
#       asymmetry comes from stagger, not from a hardcoded rule.
# ---------------------------------------------------------------------------
# Base scissor costs (tiered), calibrated so community layouts still rank sane.
SCISSOR_MI_HSB = 1.5   # index-middle, near (prefer-avoid)
SCISSOR_MI_FSB = 6.0   # index-middle, far (intolerant)
SCISSOR_MR_RP = 6.0    # middle-ring / ring-pinky (intolerant)
# USER (round 6b, X-W M/R): middle-ring and ring-middle reverse FULL-SPAN
# scissors feel IDENTICALLY extremely uncomfortable — nearly impossible, but
# just barely possible. Price every feasible full-span M/R scissor the same
# in both directions, just below the 1-row SFB weight. (Ring-pinky reverse
# full-span is a harder ruling: impossible; see impossible_rp.)
SCISSOR_MR_FSB = 5.85

# USER (round 6b, X-W): X-W with middle-index "feels practically like I-M",
# with extra effort only from the index's longer travel. The ANSI stagger
# favors the same-column top-middle / bottom-index diagonal when both fingers
# are reaching (neither key is a rest key), so discount that class. Applies to
# X-W and R-V (not E-C, whose middle key is the rest key, and not U-M, which
# is crossed against the stagger).
W_STAGGER_FAVOR = 0.35

# USER (fxw/wxf, 2026-10-23): rolling INWARD THROUGH a full-span scissor is
# not a smooth wave — the hand is fighting the geometry. Replace the IN-roll
# reward with this strain penalty (OUT-rolls on the same pair keep their
# wide-halved reward; calibrated so fxw=IMR and wxf=MII co-hold: the two
# rulings only fit together under this asymmetric split).
# Round 6b: {M,R} full-span is now direction-symmetric and gets no reward at
# all (see scorer2); this strain remains for the other adjacent pairs.
W_FULLSPAN_SCISSOR_STRAIN = 0.25

_ADJACENT = {("I", "M"), ("M", "R"), ("R", "P")}
_FORDER = {"P": 0, "R": 1, "M": 2, "I": 3, "T": 4}


def _outer_inner(f1, f2):
    if (f1, f2) in _ADJACENT or (f2, f1) in _ADJACENT:
        return (f1, f2) if _FORDER[f1] < _FORDER[f2] else (f2, f1)
    return None


def _rest_mult(name, finger, hand, up):
    """Discount scissor when the moving finger is near its rest (arched stance)."""
    rest = REST.get((hand, finger))
    if rest is None:
        d = ((ansi.cx(name) - _NATURAL_CX[hand][finger]) ** 2 +
             (ansi.cy(name) - 1.5) ** 2) ** 0.5
    else:
        d = ansi.dist(name, rest)
    # ANSI distances: rest=0, adjacent≈1.03-1.25, far>=1.3
    if d > 1.3:
        return 1.0
    if up:
        return 0.05 if d < 0.3 else 0.60
    return 0.10 if d < 0.3 else 0.70


# ---------------------------------------------------------------------------
# USER: ANSI stagger creates a true L/R asymmetry for same-column pairs:
#       "a roll where both keys are on the same column, e.g. ED on my left
#       hand isn't the same as IK on my right hand, because the keyboard's
#       horizontal stagger ... makes ED a more comfortable roll, whereas with
#       IK this same stagger accentuates the scissor."
#       True ANSI: both pairs have dx=+0.25 (uniform rightward drift, verified
#       from standard key widths). The asymmetry is SIGN vs anatomy: left
#       hand wants index RIGHT of middle (+), right hand wants index LEFT of
#       middle (-). Same-integer-col pairs: left=aligned, right=CROSSED.
# ---------------------------------------------------------------------------
W_CROSS = 0.40   # per key-width of lateral crossing (prior; refit by ratings)

# USER (Q3, nc verdict): the stagger-ALIGNED offset is invisible — nc (M-top ->
# I-home, right hand, dx=+0.25) is "completely comfortable, due to the
# horizontal ANSI stagger". Only crossing BEYOND the ~0.5-key stagger grain
# registers as crossed.
CROSS_DEADZONE = 0.50


def _cross_sign(k1, f1, k2, f2, hand):
    """0 if keys respect the hand's natural finger order laterally, else |dx|.

    For adjacent finger pairs (outer=lateral, inner=central): the inner finger
    sits toward the board center. Left hand center is to the right => inner
    key should have larger x (+). Right hand => smaller x (-). Opposite sign =
    fingers laterally swapped ("crossed") — e.g. right-hand I(7.5)->K(7.75)
    puts the index key right of the middle key, violating right-hand order.
    """
    if f1 == f2:
        return 0.0
    pair = _outer_inner(f1, f2)
    if pair is None:
        return 0.0
    outer, inner = pair
    ko, ki = (k1, k2) if f1 == outer else (k2, k1)
    dx = ansi.cx(ki) - ansi.cx(ko)
    if abs(dx) < 1e-9:
        return 0.0
    expected = 1.0 if hand == "L" else -1.0
    if dx * expected > 0:
        return 0.0
    return abs(dx)


def lateral_crossing(k1, f1, k2, f2, hand):
    """Penalty for crossed lateral finger order (0 if aligned / stagger-grain)."""
    return W_CROSS * max(0.0, _cross_sign(k1, f1, k2, f2, hand) - CROSS_DEADZONE)


def stagger_favor(k1, f1, k2, f2, hand):
    """Discount for the stagger-favored same-column full-span M/I diagonal.

    USER (round 6b): X-W with middle on W / index on X feels practically like
    I-M; the only extra effort is the index's longer travel. The favorable
    class is exactly the same-column top-middle / bottom-index pair with
    natural lateral order where BOTH fingers are reaching (neither key is a
    rest key), e.g. X-W and R-V. E-C is excluded because the middle key is
    already at rest; U-M is excluded because it is crossed against the
    stagger. Returns 0.0 or W_STAGGER_FAVOR.
    """
    if {f1, f2} != {"M", "I"}:
        return 0.0
    if KEYS[k1].hand != KEYS[k2].hand:
        return 0.0
    if KEYS[k1].col != KEYS[k2].col:
        return 0.0
    if abs(KEYS[k1].row - KEYS[k2].row) < 2:
        return 0.0
    m_k, i_k = (k1, k2) if f1 == "M" else (k2, k1)
    if KEYS[m_k].row >= KEYS[i_k].row:
        return 0.0
    if _cross_sign(k1, f1, k2, f2, hand) != 0.0:
        return 0.0
    if REST.get((hand, "M")) == m_k or REST.get((hand, "I")) == i_k:
        return 0.0
    return W_STAGGER_FAVOR


# ---------------------------------------------------------------------------
# USER (2026-10-21 calibration): the anatomical scissor model above is blind
# to a class the user CANNOT PHYSICALLY ROLL: ring/middle column-swaps and
# same-column stacks. Rulings: L->, / O->, (middle in ring's column, ring in
# middle's column) "impossible and absurd"; S->W / X->S (R and M stacked in
# one column, against hand angle AND the ANSI stagger) impossible. Any
# feasible alternative must beat these — even an SFB (user: "can't even
# physically roll").
# 2026-10-23: raised 4.0 -> 9.0 (above max SFB 8.0). At 4.0 the trigram
# winners orm/arm/fai still chose the impossible stack over the SFB option,
# violating "even an SFB" — impossible must exceed W_SFB_2ROW.
W_IMPOSSIBLE_MR = 9.0

_COL_OWNERS = {1: "P", 2: "R", 3: "M", 4: "I", 5: "I",
               6: "I", 7: "I", 8: "M", 9: "R", 10: "P"}


def impossible_mr(k1, f1, k2, f2):
    """Ring/middle column-swap or same-column stack = unphysical (USER).

    2026-10-23 (O->L ruling): a same-column stack is NOT automatically
    impossible — the ANSI stagger makes it viable when it puts the middle
    key LEFT of the ring key on the right hand ("the horizontal ansi
    stagger gives just slightly enough room to make this viable ... it
    works, isn't a SFB"). O->L with M on O / R on L: aligned + natural
    wave (middle up, ring down) => viable. S->W/X->S stacks fail on
    crossing (stagger puts them against the hand) or on row direction
    (ring up + middle down) => still impossible (USER 2026-10-21).
    """
    if {f1, f2} != {"M", "R"}:
        return False
    c1, c2 = KEYS[k1].col, KEYS[k2].col
    if _COL_OWNERS[c1] == f2 and _COL_OWNERS[c2] == f1:
        return True                                    # swap (L->, / O->,)
    if c1 != c2:
        return False
    # same-column stack: viable only if stagger-aligned AND natural direction
    hand = KEYS[k1].hand
    outer, inner = _outer_inner(f1, f2)
    ko, ki = (k1, k2) if f1 == outer else (k2, k1)
    dx = ansi.cx(ki) - ansi.cx(ko)
    expected = 1.0 if hand == "L" else -1.0
    if dx * expected < 0:
        return True                                    # against the stagger
    if (ansi.cy(ko) - 0.5) < (ansi.cy(ki) - 0.5):
        # ring up + middle down = bad direction (shorter finger up).
        # A HALF-scissor stack stays impossible (S->W / X->S rulings: the
        # rest discount would otherwise make it look cheap, ~0.2). A
        # FULL-SPAN stack (row gap >= 2) is physically reachable when the
        # stagger aligns it and is instead priced as an intolerable scissor
        # (6.0): fxw needs X->W M/R = "uncomfortable, but best possible
        # fingering I can manage" (USER 2026-10-23) — feasible, never cheap.
        return abs((ansi.cy(ko) - 0.5) - (ansi.cy(ki) - 0.5)) < 2.0
    return False


# USER (2026-10-23, wxf ruling): "index is to the right of my middle finger
# on my hand ... both of these keys, under the model's proposed fingering,
# go entirely against my hand" — index on X + middle on F crosses the finger
# order by 1.5 key widths, far beyond the 0.5-key stagger grain. That is not
# a priceable discomfort, it is impossible: any feasible alternative (even an
# SFB) must beat it.
W_CROSS_IMPOSSIBLE = 1.5   # crossed |dx| at or beyond this = unphysical


def impossible_cross(k1, f1, k2, f2):
    """Extreme lateral finger-order crossing = unphysical (USER, X/F ruling)."""
    if f1 == f2:
        return False
    pair = _outer_inner(f1, f2)
    if pair is None:
        return False
    if KEYS[k1].hand != KEYS[k2].hand:
        return False
    if _cross_sign(k1, f1, k2, f2, KEYS[k1].hand) >= W_CROSS_IMPOSSIBLE:
        return True
    return False


def impossible_ir(k1, f1, k2, f2):
    """Index/ring same-column stack = middle squeezed between them (USER,
    X->W ruling): "the index and ring squeeze my middle finger ... there's
    absolutely no point to using ring". Same-column only; crossed R/I pairs
    stay priced by scissor + ri_span."""
    if {f1, f2} != {"I", "R"}:
        return False
    return KEYS[k1].col == KEYS[k2].col


def impossible_rp(k1, f1, k2, f2):
    """Ring/pinky reverse FULL-SPAN scissor = impossible (USER round 6b).

    The bad shape is pinky up + ring down (outer/shorter finger up). The
    reverse (ring up + pinky down) is the natural wave and stays free. Only
    full-span cases are covered; half-span R/P rolls keep their sign-specific
    weights. Reachable corpus cases: Q->X PR and X->Q RP.
    """
    if {f1, f2} != {"R", "P"}:
        return False
    if KEYS[k1].hand != KEYS[k2].hand:
        return False
    if abs(KEYS[k1].row - KEYS[k2].row) < 2:
        return False
    outer, inner = _outer_inner(f1, f2)
    ko, ki = (k1, k2) if f1 == outer else (k2, k1)
    return ansi.cy(ko) < ansi.cy(ki)


# ---------------------------------------------------------------------------
# USER (O->, hand-shift conflict, 2026-10-22): index reaching into the
# middle's column drags the hand right; a ring press in its own column wants
# the hand left — "both of these keys, under ring-index, are working against
# each other". Force the parallel middle-index roll instead (O->, = M->I,
# ,->O = I->M). Fires for ring-col {2,9} x middle-col {3,8} R/I pairs.
# ---------------------------------------------------------------------------
W_HAND_SHIFT = 0.40


def hand_shift_penalty(n1, f1, n2, f2):
    if {f1, f2} != {"I", "R"}:
        return 0.0
    if KEYS[n1].hand != KEYS[n2].hand:
        return 0.0
    i_k, r_k = (n1, n2) if f1 == "I" else (n2, n1)
    if KEYS[i_k].col in (3, 8) and KEYS[r_k].col in (2, 9):
        return W_HAND_SHIFT
    return 0.0


# ---------------------------------------------------------------------------
# USER (ring-index tendon coupling, 2026-10-21): "the tendons between these
# two fingers are the ones that are the most coupled on my hands ...
# extending them in opposite lateral directions more than a single key of
# distance feels straining." Penalty per ANSI-key of lateral span BEYOND
# rest-span + 1. Priced on R/I bigrams AND across same-hand trigrams (their
# QFB ruling: judge the whole trigram — ring at Q + index at B strains even
# though no R/I bigram exists in the path).
# ---------------------------------------------------------------------------
W_RI_SPAN = 1.0


def _rest_span(hand):
    return abs(ansi.cx(REST[(hand, "R")]) - ansi.cx(REST[(hand, "I")]))


def ri_span_penalty(n1, f1, n2, f2):
    if {f1, f2} != {"R", "I"}:
        return 0.0
    hand = KEYS[n1].hand
    span = abs(ansi.cx(n1) - ansi.cx(n2))
    excess = span - (_rest_span(hand) + 1.0)
    return W_RI_SPAN * excess if excess > 0 else 0.0


W_MI_SPAN = 1.0


def mi_span_penalty(n1, f1, n2, f2):
    if {f1, f2} != {"M", "I"}:
        return 0.0
    if KEYS[n1].hand != KEYS[n2].hand:
        return 0.0
    span = abs(ansi.cx(n1) - ansi.cx(n2))
    excess = span - 2.0
    return W_MI_SPAN * excess if excess > 0 else 0.0


def scissor_cost(k1, f1, k2, f2, hand):
    """ANSI-corrected scissor cost. 0 if not a scissor.

    Same anatomical direction rule the user endorsed (shorter up = bad), but
    evaluated with true ANSI distance so left/right asymmetry is physical.
    """
    if f1 == f2:
        return 0.0
    if (impossible_mr(k1, f1, k2, f2) or impossible_ir(k1, f1, k2, f2)
            or impossible_cross(k1, f1, k2, f2)
            or impossible_rp(k1, f1, k2, f2)):
        return W_IMPOSSIBLE_MR
    pair = _outer_inner(f1, f2)
    if pair is None:
        if {f1, f2} != {"R", "I"}:
            return 0.0
        # USER (X->Q, 2026-10-23): ring on X + pinky on Q is a "reverse,
        # full scissor ... practically impossible"; the alt is index-middle.
        # IR is the same full-span diagonal (index X bottom + ring Q top):
        # price ring-index rows like MR/RP instead of returning 0. {R,I} is
        # not tendon-adjacent so it was never in _ADJACENT — the lateral
        # ri_span_penalty still covers pure spreads.
        outer, inner = "R", "I"
    else:
        outer, inner = pair
    if f1 == outer:
        outer_k, inner_k = k1, k2
    else:
        outer_k, inner_k = k2, k1
    if ({f1, f2} == {"R", "I"} and ansi.cy(outer_k) < ansi.cy(inner_k)
            and abs(ansi.cy(outer_k) - ansi.cy(inner_k)) < 2.0):
        return 0.0
    r_out = ansi.cy(outer_k) - 0.5
    r_in = ansi.cy(inner_k) - 0.5
    if r_out == r_in:
        return 0.0
    if (outer, inner) == ("M", "I"):
        is_scissor = r_in < r_out      # index up + middle down
        up_k, up_f = inner_k, inner
        dn_k, dn_f = outer_k, outer
    else:
        is_scissor = r_out < r_in      # ring/pinky up + middle/ring down
        up_k, up_f = outer_k, outer
        dn_k, dn_f = inner_k, inner
    if not is_scissor:
        return 0.0
    d = ansi.dist(k1, k2)
    # USER (O->, 2026-10-23): a FULL-SPAN scissor (top<->bottom row) is never
    # saved by an arched/rest stance — ring on O + middle on comma is "a
    # ridiculous reverse full scissor ... one of the most uncomfortable things
    # out there". Kill the rest discount for span>=2 (kills exactly the
    # RM/MR comma wins; half-scissors keep the arch discount).
    if abs(r_out - r_in) >= 2.0:
        mult = 1.0
    else:
        mult = _rest_mult(up_k, up_f, hand, up=True) * \
               _rest_mult(dn_k, dn_f, hand, up=False)
    if (outer, inner) == ("M", "I"):
        base = SCISSOR_MI_HSB if d < 1.3 else SCISSOR_MI_FSB
    elif (outer, inner) == ("R", "M") and abs(r_out - r_in) >= 2.0:
        base = SCISSOR_MR_FSB
    else:
        base = SCISSOR_MR_RP
    return base * mult


def is_full_span_scissor(k1, k2, sc):
    """True when `sc` (scissor_cost result) is a row-gap>=2 scissor."""
    return sc > 0.0 and abs(ansi.cy(k1) - ansi.cy(k2)) >= 2.0


# ---------------------------------------------------------------------------
# USER: M<->I roll comfort ladder (LEFT hand, QWERTY EDC-RFV / RFV-TGB refs).
#       Ranked most->least comfortable. This is the core of the arched-home
#       design principle made precise. Values are relative penalties (lower =
#       more comfortable). The user gave an ORDINAL ladder; we map it to a
#       monotone numeric scale.
#
#   Let mrow = middle finger's row (0 top, 1 home, 2 bottom)
#       irow = index finger's row
#
#   USER ladder (left M/I):
#     1. mrow=0 (top)   & irow=1 (home)   -> MOST optimal
#     2. mrow==irow, both home (1,1) OR both top (0,0) -> equal, next
#     3. mrow=1 (home)  & irow=2 (bot)    -> slightly less preferred
#     4. both bottom (2,2) OR mrow=0 top & irow=2 bot -> equal, slightly unpreferred
#        (below here = "uncomfortable but still comfortable")
#     5. mrow=1 home & irow=0 top  OR  mrow=2 bot & irow=1 home -> equal, uncomfortable
#     6. mrow=2 bot & irow=0 top -> EXTREMELY uncomfortable (worst)
#
#   Numeric scale (penalty, lower=better). Gaps mirror the user's wording:
#   "most optimal" / "equal" / "slightly less" / "slightly more unpreferred" /
#   "uncomfortable" / "extremely uncomfortable".
# ---------------------------------------------------------------------------
_MI_LADDER = {
    (0, 1): 0.00,   # top-middle, home-index  (most optimal)
    (1, 1): 0.05,   # both home
    (0, 0): 0.05,   # both top
    (1, 2): 0.15,   # home-middle, bottom-index (slightly less)
    (2, 2): 0.30,   # both bottom
    (0, 2): 0.30,   # top-middle, bottom-index
    (1, 0): 0.55,   # home-middle, top-index (uncomfortable)
    (2, 1): 0.55,   # bottom-middle, home-index (uncomfortable)
    (2, 0): 0.90,   # bottom-middle, top-index (extremely uncomfortable)
}


def mi_roll_penalty(k1, f1, k2, f2, hand):
    """Roll comfort penalty for an M<->I bigram, using the USER ladder.

    Returns (penalty, is_roll_in). penalty is the ladder value for the
    (middle-row, index-row) pair; direction (roll_in vs roll_out) is handled
    separately by the roll weights. For the LEFT hand this is the user's exact
    ladder. For the RIGHT hand we use the same ladder on the ANSI-corrected
    rows — the ladder is anatomical (finger relative heights), not
    hand-specific, but the DISTANCE/scissor terms still differ by hand via
    ansi geometry.
    """
    if {f1, f2} != {"M", "I"}:
        return None
    if f1 == "M":
        m_k, i_k = k1, k2
    else:
        m_k, i_k = k2, k1
    mrow = int(round(ansi.cy(m_k) - 0.5))
    irow = int(round(ansi.cy(i_k) - 0.5))
    return _MI_LADDER.get((mrow, irow), 0.5)


# Roll direction weights. USER: rolls are desirable; in-roll > out-roll.
# (These are the only remaining "borrowed" magnitudes; the DIRECTION preference
# is universal. The M/I ladder above supersedes flat roll for M/I pairs.)
W_ROLL_IN = -0.35
W_ROLL_OUT = -0.20
# USER (round 6, X->F): wide lateral gap is no smooth roll. 1.75 catches the
# 1.5-col x 1-row diagonal class (X<->F at dist 1.803) so X->F prefers
# ring-index (RI) over the cross-column in-roll RM. 2.0 left X->F's in-roll
# full-strength and RM won.
W_ROLL_WIDE_GAP = 1.75
# USER (Q4, leaps): "Uncomfortable" ('he' = pinky->index gap-3, test-typed and
# flagged). BUT their other datapoint: 'st' (ring->index, gap-2) = "a regular
# bigram roll ... isn't ideal, but still could be worse" -> mild. So tier by
# gap: gap2 mild, gap3 (only P<->I) bad, +pinky extra on top ('he' = 0.50).
W_ROLL_LEAP_GAP2 = 0.15
W_ROLL_LEAP_GAP3 = 0.35
PINKY_LEAP_EXTRA = 0.15
# USER (Q4, pinky philosophy): "alternation-first, roll-second" for pinkies —
# occasional pinky roll is fine but pinkies should not work like IMR roll
# engines. Discount (not remove) roll rewards that involve a pinky.
PINKY_ROLL_DISCOUNT = 0.4

# USER (round 6, roll directions): the in>out rule is NOT universal.
#   * M<->I: "rolls on either direction are equally as comfortable" -> use the
#     average of the IN/OUT rewards for both directions.
#   * M<->R: ring->middle (IN) slightly better than middle->ring — already
#     satisfied by W_ROLL_IN/W_ROLL_OUT, unchanged.
#   * R<->P: "significantly more comfortable to roll ring-pinky than
#     pinky-ring" -> the pinky pair INVERTS in>out. R->P is the good direction
#     (ring pushes out to pinky); P->R is a mild penalty.
W_ROLL_MI_SYM = (W_ROLL_IN + W_ROLL_OUT) / 2.0   # -0.275, both directions
W_ROLL_RING_PINKY = W_ROLL_IN * PINKY_ROLL_DISCOUNT   # -0.14, R->P (good)
W_ROLL_PINKY_RING = 0.06                              # +0.06, P->R (awkward)

W_ALT = -0.12         # hand alternation (unchanged; alternation-first = pinky-friendly)

# SFB weights (row-weighted). Still somewhat prior, but the ANSI distance now
# modulates same-column SFBs that the integer grid treated as free.
W_SFB_SAME_ROW = 5.0
W_SFB_1ROW = 6.0
# USER (round 6b): after the X-W stagger-favor discount, the fxw=IMR vs
# wxf=MII pair only co-holds if the DOWNWARD 1-row SFB (home->bottom, e.g.
# F->X inside fxw) is the more expensive of the two. A downward reach is a
# bigger posture break than the reverse; +0.3 is the smallest offset that
# preserves both hard checks with margin.
W_SFB_1ROW_DOWN = 6.3
W_SFB_2ROW = 8.0

# Trigram weights.
# USER (Q5, redirects): "you should only consider weak redirects for the pinky
# only - my index/middle/ring fingers are basically equally strong". Same-hand
# direction reversal = mild if it stays in IMR; expensive if a pinky takes part
# in the turn (the 'abe' class: pinky-index weak redirect).
W_REDIRECT = 0.30        # IMR-only reversal (mild)
W_REDIRECT_WEAK = 1.80   # pinky-involved reversal
W_CONTINUE = -0.10
W_SFS_1U = 2.50
W_SFS_FAR = 3.50

# USER (Q5, 'abe'): "Are there no extra considerations ... so that trigrams
# that feel truly awful, like 'abe', are considered further?" — stacked
# weaknesses are super-additive: if a trigram carries >=3 distinct flaw flags
# (pinky-redirect / scissor / leap / against-the-stagger-grain), charge an
# extra awfulness bonus on top of the component costs.
W_AWFUL_TRIGRAM = 1.50
AWFUL_THRESHOLD = 3

def _against_grain(k1, k2):
    """True if the bigram moves AGAINST the ANSI grain (down-left or up-right).

    Grain: home/bot rows drift rightward (+0.25/+0.75), so 'with the grain'
    = down-right or up-left. 'abe' W->Z is down-LEFT = against stagger.
    """
    dy = ansi.cy(k2) - ansi.cy(k1)
    dx = ansi.cx(k2) - ansi.cx(k1)
    if abs(dy) < 1e-9 or abs(dx) < 1e-9:
        return False
    return (dy > 0 and dx < 0) or (dy < 0 and dx > 0)


def awful_trigram_flags(n1, f1, n2, f2, n3, f3):
    """Count distinct weakness classes in a same-hand letter trigram."""
    flags = 0
    # (1) pinky involved in a direction reversal
    o = (_FORDER[f1], _FORDER[f2], _FORDER[f3])
    d1, d2 = o[1] - o[0], o[2] - o[1]
    if d1 * d2 < 0 and "P" in (f1, f2, f3):
        flags += 1
    grain = False
    for a, b, fa, fb in ((n1, n2, f1, f2), (n2, n3, f2, f3)):
        if KEYS[a].hand != KEYS[b].hand:
            continue
        if scissor_cost(a, fa, b, fb, KEYS[a].hand) > 0.15:
            flags += 1
        if fa != fb:
            of1, of2 = _FORDER[fa], _FORDER[fb]
            if abs(of1 - of2) >= 2:
                flags += 1
        if _against_grain(a, b):
            grain = True
    if grain:
        flags += 1
    return flags


# Balance: USER (Q8): "Evenness between index/middle/ring, index/middle favored
# over ring ... I value flow above balance" + pinky philosophy (Q4): pinky
# "alternation-first, roll-second", lower stamina — pinky gets MID-frequency
# letters (they prefer 'A' on pinky-rest over 'Z'/'X'), NOT the crown jewels,
# and carries clearly less load than IMR. So: IMR ~even with a slight IM>R
# tilt, pinky capped low. Weak weight stays (flow dominates).
W_BALANCE_EVEN = 0.4
FLAT_SHARE = {"P": 0.18, "R": 0.25, "M": 0.28, "I": 0.29}
W_HAND_BALANCE = 0.5  # want ~50/50 hands
