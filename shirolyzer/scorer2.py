"""
scorer2.py — ANSI + user-calibrated drop-in for scorer.py.

Same interface (get_tables, Corpus, SPACE) so search2.py can use it unchanged.

Corrections vs the borrowed scorer.py:
  1. Same-key repeats (ll, tt) charged a small SEPARATE penalty (W_REPEAT),
     checked BEFORE fingering — closes the DP finger-swap exploit where mixed
     fingers on one key farmed roll rewards. Industry treats doubles as their
     own class, never pooled into SFB.
  2. Effort = pure geometric travel in ANSI space (no finger-strength bias;
     user: all fingers equal).
  3. M<->I bigrams priced by the USER's row-pair comfort ladder (arched-home
     principle made precise) + direction roll reward.
  4. Lateral-crossing penalty for adjacent-finger pairs whose keys violate the
     hand's natural finger order (the ED-vs-IK / nc asymmetry; user-flagged).
     Crossed pairs also get their roll reward suppressed (no smooth roll
     across swapped fingers).
  5. Distances (scissor tiers, SFS, wide-gap roll halving) in true ANSI space.
  6. Balance: FLAT 0.25 target, weak weight (user: fingers equal, even load
     is a plus, not a driver).
"""
import numpy as np

from fingers import (KEYS, KEY_ORDER, SPACE, FORDER)
from comfort import (travel_cost, scissor_cost, is_full_span_scissor,
                     lateral_crossing, stagger_favor, mi_roll_penalty,
                     awful_trigram_flags,
                     ri_span_penalty, mi_span_penalty, hand_shift_penalty,
                     W_REPEAT, W_ROLL_IN, W_ROLL_OUT,
                     W_ROLL_MI_SYM, W_ROLL_RING_PINKY, W_ROLL_PINKY_RING,
                     W_ROLL_LEAP_GAP2, W_ROLL_LEAP_GAP3,
                     PINKY_LEAP_EXTRA, PINKY_ROLL_DISCOUNT,
                     W_AWFUL_TRIGRAM, AWFUL_THRESHOLD,
                     W_ALT, W_SFB_SAME_ROW, W_SFB_1ROW, W_SFB_1ROW_DOWN,
                     W_SFB_2ROW,
                     W_REDIRECT, W_REDIRECT_WEAK, W_CONTINUE,
                     W_SFS_1U, W_SFS_FAR, TRAVEL_SCALE,
                     W_BALANCE_EVEN, FLAT_SHARE, W_HAND_BALANCE,
                     W_FULLSPAN_SCISSOR_STRAIN, W_ROLL_WIDE_GAP)
import ansi

NKEYS = len(KEY_ORDER)          # 30
SPACE_ID = NKEYS                # pseudo-key id for space
N = NKEYS + 1                   # 31

W_REPEAT_SWAP = 0.5   # extra: double typed with DIFFERENT fingers (finger swap
                      # mid-double) — physically awkward, DP must not farm it.
W_REPEAT_SWAP_TRIGRAM = 0.5
SPACE_EFFORT = 0.6    # thumb press (no travel model needed)

FID = {"P": 0, "R": 1, "M": 2, "I": 3, "T": 4}


def _finger_ids(name, hand):
    if name == SPACE:
        return [("T", 0.0)]
    return list(KEYS[name].alts)


_ALT_CONTEXTS = {
    ("X", "M"): {"C"},
    ("W", "M"): {"Q"},
    ("L", "I"): {"O"},
}
_ALT_BLOCKS = {
    ("J", "M"): {"T", "I", "G"},
    ("M", "M"): {"J"},
    ("C", "M"): {"R"},
    ("G", "M"): None,
    ("V", "M"): {"G"},
}
W_ALT_CONTEXT = 12.0
W_CR_REVERSE = 3.60   # USER 2026-10-25: v@C->d@R index/middle full-span reverse scissor


def _alt_context_penalty(name, finger, neighbours):
    if name == SPACE or finger == KEYS[name].alts[0][0]:
        return 0.0
    if (name, finger) in _ALT_BLOCKS:
        blocked = _ALT_BLOCKS[(name, finger)]
        if blocked is None or any(k in blocked for k in neighbours):
            return W_ALT_CONTEXT
    allowed = _ALT_CONTEXTS.get((name, finger))
    if allowed is not None and not any(k in allowed for k in neighbours):
        return W_ALT_CONTEXT
    return 0.0


def _same_hand_bigram(n1, f1, n2, f2):
    k1, k2 = KEYS[n1], KEYS[n2]
    hand = k1.hand
    if f1 == f2:
        # SFB (different keys, same finger). ANSI row for weighting.
        dr = abs(ansi.row(n1) - ansi.row(n2))
        if dr == 0:
            return W_SFB_SAME_ROW
        if dr == 1:
            # USER (round 6b): downward 1-row SFB (row index increases) is
            # slightly worse than the upward reach.
            return (W_SFB_1ROW_DOWN if ansi.row(n2) > ansi.row(n1)
                    else W_SFB_1ROW)
        return W_SFB_2ROW
    sc = scissor_cost(n1, f1, n2, f2, hand)
    cross = lateral_crossing(n1, f1, n2, f2, hand)
    o1, o2 = FORDER[f1], FORDER[f2]
    lad = 0.0
    if {f1, f2} == {"M", "I"}:
        v = mi_roll_penalty(n1, f1, n2, f2, hand)
        lad = v if v is not None else 0.0
    mi_span = mi_span_penalty(n1, f1, n2, f2)
    pair = {f1, f2}
    full_sc = is_full_span_scissor(n1, n2, sc)
    if abs(o2 - o1) >= 2:
        # USER Q4: leaps are penalties now ('he' flagged), tiered by gap.
        base = W_ROLL_LEAP_GAP3 if abs(o2 - o1) >= 3 else W_ROLL_LEAP_GAP2
        reward = base + (PINKY_LEAP_EXTRA if "P" in (f1, f2) else 0.0)
    else:
        # USER (round 6): direction rules are per finger-pair, not universal.
        if pair == {"M", "I"}:
            reward = W_ROLL_MI_SYM       # equal both directions
        elif pair == {"P", "R"}:
            # ring->pinky (out) >> pinky->ring (in): inverted pinky pair
            reward = (W_ROLL_RING_PINKY if f2 == "P"
                      else W_ROLL_PINKY_RING)
        else:
            reward = W_ROLL_IN if o2 > o1 else W_ROLL_OUT
        if ansi.dist(n1, n2) >= W_ROLL_WIDE_GAP:
            reward *= 0.5          # wide lateral gap is no smooth roll
        if cross > 0:
            reward *= 0.25         # crossed fingers don't roll smoothly
        if "P" in (f1, f2) and pair != {"P", "R"}:
            reward *= PINKY_ROLL_DISCOUNT  # pinky: roll is a bonus, not a job
    # USER (fxw/wxf, 2026-10-23): rolling INWARD through a FULL-SPAN scissor
    # is the hand fighting the geometry (RMI on wxf) — kill the IN-roll
    # bounty. OUT-rolls on the same physical pair (IMR on fxw, typed the
    # other way) keep their (wide-halved) reward so fxw's IMR stays viable
    # while wxf's MII (SFB on 2-3) beats RMI (full-span scissor on 1-2).
    # Asymmetric on purpose: the user's two trigram rulings only co-hold
    # under this split. USER (round 6): M/I rolls are direction-symmetric,
    # so the asymmetry excludes {M,I} (their rolls keep the symmetric reward).
    if pair == {"M", "R"} and full_sc:
        # USER (round 6b): middle-ring and ring-middle reverse full-span
        # scissors are IDENTICALLY extremely uncomfortable. No direction-
        # specific roll reward/strain; the scissor base is symmetric.
        reward = 0.0
    elif o2 > o1 and pair != {"M", "I"} and full_sc:
        reward = W_FULLSPAN_SCISSOR_STRAIN
    sf = stagger_favor(n1, f1, n2, f2, hand)
    ri = ri_span_penalty(n1, f1, n2, f2)
    hs = hand_shift_penalty(n1, f1, n2, f2)
    return sc + cross + lad + mi_span + ri + hs + reward - sf


def _bigram_cost(n1, f1, n2, f2):
    """Full bigram transition cost (finger-pair aware)."""
    if n1 == SPACE and n2 == SPACE:
        return 0.0
    if n1 == SPACE or n2 == SPACE:
        return W_ALT           # thumb alternation-ish; effort charged separately
    if n1 == n2:
        # Same-key repeat: separate small class (unavoidable doubles), and
        # mixed-finger doubles are physically worse. Checked FIRST so the DP
        # can't farm roll rewards on one key.
        return W_REPEAT + (W_REPEAT_SWAP if f1 != f2 else 0.0)
    k1, k2 = KEYS[n1], KEYS[n2]
    if k1.hand != k2.hand:
        return W_ALT
    cost = _same_hand_bigram(n1, f1, n2, f2)
    if {n1, n2} == {"C", "R"} and {f1, f2} == {"I", "M"}:
        cost += W_CR_REVERSE
    return cost


def _trigram_cost(n1, f1, n2, f2, n3, f3):
    """Trigram-only costs (SFS, redirects, continuation)."""
    cost = 0.0
    if n1 == n3 and f1 != f3:
        cost += W_REPEAT_SWAP_TRIGRAM
    if n1 == "M" and f2 == "M" and (n2 == "J" or n3 == "J"):
        cost += W_ALT_CONTEXT
    if n2 == "M" and f3 == "M" and n3 == "J":
        cost += W_ALT_CONTEXT
    if ({n1, n3} == {"C", "R"} and f1 != f3 and
            "I" in (f1, f3)):
        cost += W_CR_REVERSE
    # SFS: same finger on keys 1 & 3 (different key), ANSI distance tiering
    if n1 != SPACE and n3 != SPACE and n1 != n3 and f1 == f3:
        k1, k3 = KEYS[n1], KEYS[n3]
        mid_same_finger = (
            n2 != SPACE and f2 == f1 and KEYS[n2].hand == k1.hand
        )
        if k1.hand == k3.hand and not mid_same_finger:
            d = ansi.dist(n1, n3)
            cost += W_SFS_1U if d <= 1.3 else W_SFS_FAR
    ns = (n1, n2, n3)
    if SPACE in ns:
        return cost
    ks = (KEYS[n1], KEYS[n2], KEYS[n3])
    if ks[0].hand != ks[1].hand or ks[1].hand != ks[2].hand:
        return cost
    # USER QFB ruling (full-trigram ring-index strain): when ring and index
    # sit at trigram positions 1 and 3 (no R/I bigram to price the span),
    # the tendon-coupling span penalty must still apply.
    trio = ((n1, f1, 0), (n2, f2, 1), (n3, f3, 2))
    r_pos = next((t for t in trio if t[1] == "R"), None)
    i_pos = next((t for t in trio if t[1] == "I"), None)
    if r_pos is not None and i_pos is not None and abs(r_pos[2] - i_pos[2]) == 2:
        cost += ri_span_penalty(r_pos[0], "R", i_pos[0], "I")
    o = (FORDER[f1], FORDER[f2], FORDER[f3])
    d1, d2 = o[1] - o[0], o[2] - o[1]
    if d1 == 0 or d2 == 0:
        return cost  # same-finger involvement handled elsewhere
    if d1 * d2 < 0:  # direction reversal = redirect
        # USER Q5: only PINKY-in-the-turn reversals are the expensive class;
        # IMR reversals are mild (all three fingers equally strong).
        pinky_involved = "P" in (f1, f2, f3)
        cost += W_REDIRECT_WEAK if pinky_involved else W_REDIRECT
        # USER Q5 'abe': stacked weaknesses are super-additive.
        if (pinky_involved and
                awful_trigram_flags(n1, f1, n2, f2, n3, f3) >= AWFUL_THRESHOLD):
            cost += W_AWFUL_TRIGRAM
    else:
        cost += W_CONTINUE
    return cost


# ---------------------------------------------------------------- tables
def build_tables():
    """BIG[i,j] = min cost bigram key_i->key_j; TRI[i,j,k] likewise.
    Includes per-key alt premiums and keypress effort."""
    BIG = np.zeros((N, N), dtype=np.float64)
    TRI = np.zeros((N, N, N), dtype=np.float64)
    BIG_F = np.zeros((N, N), dtype=np.int8)
    TRI_F = np.zeros((N, N, N, 3), dtype=np.int8)

    names = KEY_ORDER + [SPACE]
    eff = {}
    for n in names:
        if n == SPACE:
            eff[(n, "T")] = SPACE_EFFORT
            continue
        for f, prem in KEYS[n].alts:
            eff[(n, f)] = travel_cost(n, f, KEYS[n].hand) + TRAVEL_SCALE * prem

    for i, n1 in enumerate(names):
        opts1 = _finger_ids(n1, KEYS[n1].hand if n1 in KEYS else "R")
        for j, n2 in enumerate(names):
            opts2 = _finger_ids(n2, KEYS[n2].hand if n2 in KEYS else "R")
            best = None
            for f1, p1 in opts1:
                for f2, p2 in opts2:
                    if n1 == n2 and f1 != f2:
                        continue
                    c = (eff[(n1, f1)] + eff[(n2, f2)] +
                         _alt_context_penalty(n1, f1, (n2,)) +
                         _alt_context_penalty(n2, f2, (n1,)) +
                         _bigram_cost(n1, f1, n2, f2))
                    if best is None or c < best[0]:
                        best = (c, FID[f1], FID[f2])
            BIG[i, j] = best[0]
            BIG_F[i, j] = best[1] * 5 + best[2]

    for i, n1 in enumerate(names):
        opts1 = _finger_ids(n1, KEYS[n1].hand if n1 in KEYS else "R")
        for j, n2 in enumerate(names):
            opts2 = _finger_ids(n2, KEYS[n2].hand if n2 in KEYS else "R")
            for k, n3 in enumerate(names):
                opts3 = _finger_ids(n3, KEYS[n3].hand if n3 in KEYS else "R")
                best = None
                for f1, p1 in opts1:
                    for f2, p2 in opts2:
                        for f3, p3 in opts3:
                            if ((n1 == n2 and f1 != f2) or
                                    (n2 == n3 and f2 != f3) or
                                    (n1 == n3 and f1 != f3)):
                                continue
                            c = (eff[(n1, f1)] + eff[(n2, f2)] + eff[(n3, f3)] +
                                 _alt_context_penalty(n1, f1, (n2, n3)) +
                                 _alt_context_penalty(n2, f2, (n1, n3)) +
                                 _alt_context_penalty(n3, f3, (n1, n2)) +
                                 _bigram_cost(n1, f1, n2, f2) +
                                 _bigram_cost(n2, f2, n3, f3) +
                                 _trigram_cost(n1, f1, n2, f2, n3, f3))
                            if best is None or c < best[0]:
                                best = (c, FID[f1], FID[f2], FID[f3])
                TRI[i, j, k] = best[0]
                TRI_F[i, j, k] = best[1:]
    return BIG, TRI, BIG_F, TRI_F


TABLES = None


def get_tables():
    global TABLES
    if TABLES is None:
        TABLES = build_tables()
    return TABLES


# ---------------------------------------------------------------- default
def build_default_tables():
    """Standard (no-alt) fingering = default finger only."""
    BIGd = np.zeros((N, N))
    TRI_d = np.zeros((N, N, N))
    names = KEY_ORDER + [SPACE]

    def dflt(n):
        if n == SPACE:
            return "T"
        return KEYS[n].alts[0][0]

    eff = {}
    for n in names:
        if n == SPACE:
            eff[n] = SPACE_EFFORT
        else:
            eff[n] = travel_cost(n, dflt(n), KEYS[n].hand)
    for i, n1 in enumerate(names):
        for j, n2 in enumerate(names):
            f1, f2 = dflt(n1), dflt(n2)
            BIGd[i, j] = eff[n1] + eff[n2] + _bigram_cost(n1, f1, n2, f2)
            for k, n3 in enumerate(names):
                f3 = dflt(n3)
                TRI_d[i, j, k] = (BIGd[i, j] + eff[n3] +
                                  _bigram_cost(n2, f2, n3, f3) +
                                  _trigram_cost(n1, f1, n2, f2, n3, f3))
    return BIGd, TRI_d


DEFAULT_TABLES = None


def get_default_tables():
    global DEFAULT_TABLES
    if DEFAULT_TABLES is None:
        DEFAULT_TABLES = build_default_tables()
    return DEFAULT_TABLES


# ---------------------------------------------------------------- corpus
# Reuse scorer.Corpus unchanged (pure n-gram bookkeeping, no model assumptions)
from scorer import Corpus  # noqa: E402,F401
