"""
fingers.py — Physical fingering model for the custom arched-home layout design.

Encodes the design spec (finger/alt-finger assignments, conditional roll
rules, scissors) into a machine-checkable model.

Position labels use QWERTY keys as *physical position names only*:

    top:   Q W E R T | Y U I O P      (row 0, cols 1..10)
    home:  A S D F G | H J K L ;      (row 1, cols 1..10)
    bot:   Z X C V B | N M , . /      (row 2, cols 1..10)

Rest (home) position: A W E F | J I O ;
  left pinky->A, left ring->W, left middle->E, left index->F
  right index->J, right middle->I, right ring->O, right pinky->;

Finger codes: P=pinky R=ring M=middle I=index  (per hand; hand derived from col)
Thumb: T (space).

Alt-finger sets below are the consolidation of the base fingering rules +
trigram-context inferences (rules Y/Z). Each alt carries a small "premium" (extra effort) so that, all else
being equal, standard fingering is preferred (rule T), while alts remain
available to kill SFBs and create rolls (the entire point of the design).
"""
from dataclasses import dataclass

# ---------------------------------------------------------------- geometry
ROWS = {"top": 0, "home": 1, "bot": 2}
QWERTY_TOP = "QWERTYUIOP"
QWERTY_HOME = "ASDFGHJKL;"
QWERTY_BOT = "ZXCVBNM,./"
KEY_ORDER = list(QWERTY_TOP) + list(QWERTY_HOME) + list(QWERTY_BOT)  # 30 keys
KEY_INDEX = {k: i for i, k in enumerate(KEY_ORDER)}

# Optional ISO-102 key (left of Z on ISO boards) — punctuation candidate only.
ISO_KEY = "ISO"


@dataclass(frozen=True)
class Key:
    name: str          # qwerty position label, e.g. "E"
    row: int           # 0 top, 1 home, 2 bot
    col: int           # 1..10
    hand: str          # "L" or "R"
    rest_finger: str   # finger that rests on this key, or "" if not a rest key
    alts: tuple        # tuple of (finger, premium) — includes default finger w/ premium 0


def _mk(name, row, col, alts, rest=""):
    hand = "L" if col <= 5 else "R"
    return Key(name, row, col, hand, rest, tuple(alts))


# Premiums: 0 = default finger. ~0.15-0.30 = "natural" alt (1-key reach or own
# stretch column). ~0.5-0.8 = long reach (2 keys). These are small compared to
# SFB/scissor penalties, but break ties toward standard fingering.
KEYS = {}
for _k in [
    # ---------------- LEFT HAND ----------------
    # rule 3 (USER 2026-10-21 correction): Q pressed with RING by default;
    # MIDDLE alt for Q<->S index/middle rolls (isolated-bigram preference);
    # PINKY alt = trigram escape only (user's natural QFB = pinky-middle-index
    # with hand tilt — premium keeps it out of normal picks).
    _mk("Q", 0, 1, [("R", 0.0), ("M", 0.20), ("P", 2.40)]),
    # USER (round 6, X->W): natural standalone fingering is middle-index
    # (M on X, I on W). W needs the index alt for that pairing / its triggers;
    # 0.55 mirrors X's index alt (2-key reach class).
    _mk("W", 0, 2, [("R", 0.0), ("M", 0.20), ("I", 2.00)], rest="R"),
    # rule 2: E middle (rest). rule Y example: E may take RING in trigrams
    # (e.g. ERF -> ring-middle-index roll), so ring alt allowed (1-key reach).
    _mk("E", 0, 3, [("M", 0.0), ("R", 0.25)], rest="M"),
    # rule 2: RT middle default, index alt (ER/RE, RT/TR rolls).
    # ring alt = mirror of U (ROLLS 11-12); 0.15 < U's 0.20 so left (RA/AR)
    # wins the tie.
    _mk("R", 0, 4, [("M", 0.0), ("I", 0.05), ("R", 0.15)]),
    _mk("T", 0, 5, [("M", 0.0), ("I", 0.15)]),
    # rule 1+5: A pinky rest. USER (2026-10-21): RING alt exists for
    # modifier-held rolls (AW becomes ring->middle when shift/ctrl occupies
    # the pinky) and enables Z->A pinky->ring (semi-comfortable). Pinky-A is
    # the unmodified default, so the premium is high: ring-A must only beat
    # SFB-class alternatives (the scorer has no modifier context).
    # USER (2026-10-24): ring-on-; is available for the joi class
    # (j@M -> o@; -> i@O as I-R-M).
    _mk("A", 1, 1, [("P", 0.0), ("R", 2.20)], rest="P"),
    # rule 5: S ring default. rule 6: S may take index; inference: middle alt
    # (1-key reach) enables ring-middle rolls that fix the Q-S SFB class.
    _mk("S", 1, 2, [("R", 0.0), ("M", 0.25), ("I", 0.55)]),
    # rule 5: D middle default. rule 6: D index alt (1-key reach; fixes e-d, W-D class).
    _mk("D", 1, 3, [("M", 0.0), ("I", 0.20)]),
    # rule 1: F index rest. Inference: middle alt (1-key reach) for trigram
    # escapes (e.g. EDF -> middle-index-middle).
    _mk("F", 1, 4, [("I", 0.0), ("M", 0.25)], rest="I"),
    # rule 5+6: G index default; middle alt allowed as far-reach escape.
    _mk("G", 1, 5, [("I", 0.0), ("M", 0.60)]),
    # rule 10: Z pinky bottom.
    _mk("Z", 2, 1, [("P", 0.0)]),
    # rule 9: X ring default, middle alt (XC/CX rolls), index alt (rule 10).
    _mk("X", 2, 2, [("R", 0.0), ("M", 0.25), ("I", 2.00)]),
    # rule 8: C index default, middle alt (CV/VC rolls).
    _mk("C", 2, 3, [("I", 0.0), ("M", 0.25)]),
    # rule 7: VB index, middle alt (VB/BV, and CV chain).
    _mk("V", 2, 4, [("I", 0.0), ("M", 0.25)]),
    _mk("B", 2, 5, [("I", 0.0), ("M", 0.25)]),
    # ---------------- RIGHT HAND ----------------
    # rule 7: NM index; M middle alt (NM/MM rolls).
    _mk("N", 2, 6, [("I", 0.0)]),
    _mk("M", 2, 7, [("I", 0.0), ("M", 0.25)]),
    # rule 9: comma ring default, index alt (,M/M,). inference: middle alt
    # (mirror of C) as escape.
    _mk(",", 2, 8, [("R", 0.0), ("I", 0.25), ("M", 0.55)]),
    # rule 10 (USER 2026-10-21 correction): PERIOD = RING default, not pinky
    # (mirror of X). Alts mirror X's set (M/I). Pinky never touches period;
    # slash stays pinky (mirror of Z).
    _mk(".", 2, 9, [("R", 0.0), ("M", 0.25), ("I", 0.55), ("P", 0.35)]),
    # rule 10: slash pinky.
    _mk("/", 2, 10, [("P", 0.0)]),
    # rule 5: HJ index. J middle alt = mirror of F (ROLLS 1-4); premium 0.90
    # offsets ANSI geometry so left (FG/GF) wins ties and right (JN/NJ) wins ties.
    _mk("H", 1, 6, [("I", 0.0)]),
    _mk("J", 1, 7, [("I", 0.0), ("M", 0.90)], rest="I"),
    # rule 5+6: K middle default, index alt (OK/KO class + IK/SFT escapes).
    # USER (2026-10-25): ring-on-K is comfortable and preferred for K->J->M
    # (hey) to avoid the K-M middle half-SFB.
    _mk("K", 1, 8, [("M", 0.0), ("I", 0.20), ("R", 0.30)]),
    # rule 5: L ring default. inference: middle alt (mirror of S).
    # USER (round 6, O->L): MI is the PREFERRED default; MR is only the
    # fallback used to dodge an SFB in a trigram ("middle and ring get very
    # clamped together"). Index-on-L is a 2-key reach, so the usual +0.55
    # premium would keep MR ahead; a NEGATIVE premium encodes the user's
    # stated preference (negative = preferred alt, not extra effort). At
    # -0.75, O->L picks MI (2.215) by 0.015 over MR (2.230) after the M/I
    # roll-symmetry change.
    _mk("L", 1, 9, [("R", 0.0), ("M", 0.25), ("I", -0.75)]),
    _mk(";", 1, 10, [("P", 0.0), ("R", 0.05)], rest="P"),
    # rule 2: YUI middle. U/I ring alt (UI/IU, IO/OI). Inference (mirror of
    # T->index): Y/U index alt (own stretch columns). I ring alt (rule 2).
    _mk("Y", 0, 6, [("M", 0.0), ("I", 0.15)]),
    _mk("U", 0, 7, [("M", 0.0), ("R", 1.00), ("I", 1.00)]),
    _mk("I", 0, 8, [("M", 0.0), ("R", -0.70)], rest="M"),
    # rule 3: OP ring default; O middle alt (OP/PO rolls).
    _mk("O", 0, 9, [("R", 0.0), ("M", 0.20)], rest="R"),
    _mk("P", 0, 10, [("R", 0.0)]),
] :
    KEYS[_k.name] = _k

# Thumb space key (fixed).
SPACE = "@space"  # pseudo key name

# Finger lateral order for roll direction: pinky=0 ring=1 middle=2 index=3
FORDER = {"P": 0, "R": 1, "M": 2, "I": 3, "T": 4}
FINGER_NAMES = ["P", "R", "M", "I"]

# Strength (effort base per press) — SteveP-ish, ring bumped for arch model.
STRENGTH = {"P": 1.8, "R": 1.4, "M": 1.1, "I": 1.0, "T": 0.6}

# Ideal finger usage share per hand (fraction of one hand's keypresses).
# index strongest -> most; pinky least. Calibrated vs community layouts.
# Calibrated from default-finger letter shares of modern layouts
# (Colemak-DH, Canary, Gallium) measured under THIS finger model, both
# hands pooled: P 13.7% R 22.6% M 31.6% I 32.1%.
IDEAL_SHARE = {"P": 0.137, "R": 0.226, "M": 0.316, "I": 0.321}


def rest_of(finger, hand):
    """Rest key name of a finger on a hand, or None."""
    for k in KEYS.values():
        if k.hand == hand and k.rest_finger == finger:
            return k.name
    return None


REST = {(k.hand, k.rest_finger): k.name for k in KEYS.values() if k.rest_finger}


def cheb(a, b):
    (r1, c1), (r2, c2) = a, b
    return max(abs(r1 - r2), abs(c2 - c1))


def travel(key: Key, finger: str) -> float:
    """Travel cost of pressing `key` with `finger` (0 at rest).

    Downward rows cost more than upward (uncurl-up is cheap, per Engram-style
    comfort ranking); lateral reach costs most.
    """
    hand = key.hand
    rest = REST.get((hand, finger))
    if rest is None:
        # finger has no rest on this hand (e.g. middle alt on left bottom):
        # measure from the finger's *natural column* on the home row.
        natural_col = {"L": {"P": 1, "R": 2, "M": 3, "I": 4},
                       "R": {"I": 7, "M": 8, "R": 9, "P": 10}}[hand][finger]
        natural_row = 1
        dr, dc = key.row - natural_row, key.col - natural_col
    else:
        rk = KEYS[rest]
        dr, dc = key.row - rk.row, key.col - rk.col
    down = max(dr, 0) * 0.70
    up = max(-dr, 0) * 0.40
    lat = abs(dc) * 1.00
    return STRENGTH[finger] * (1.0 + down + up + lat)


def key_effort(name: str, finger: str) -> float:
    if name == SPACE:
        return STRENGTH["T"]
    return travel(KEYS[name], finger)


# ---------------------------------------------------------------- scissors
# Wiki model, validated against canonical QWERTY scissors CR, DR, KO, PL:
#   wiki-scissor = adjacent fingers, different cols, rows differ, and the
#   SHORTER finger is UP while the LONGER finger is DOWN.
#   (index-middle: index up + middle down;  middle-ring: ring up + middle
#   down;  ring-pinky: pinky up + ring down)
# Distances: HSB = Chebyshev 1, FSB = Chebyshev >= 2.
#
# Tolerance tiers:
#   index-middle wiki-type: HSB -> prefer-avoid; FSB -> intolerant.
#   index-middle reverse (middle up, index down): perfectly tolerated.
#   middle-ring / ring-pinky wiki-type: intolerant at any distance.
# Rest discount (arched home): if the UP finger is on/near its rest key, the
# posture is the natural stance (e.g. W-D, K-O under this model) and the
# scissor cost is heavily discounted.

ADJACENT = {("I", "M"), ("M", "R"), ("R", "P")}  # unordered adjacent pairs


def _outer_inner(f1, f2):
    """Return (outer, inner) for an adjacent pair, or None.
    Outer = more lateral finger (lower FORDER); inner = more central."""
    if (f1, f2) in ADJACENT or (f2, f1) in ADJACENT:
        return (f1, f2) if FORDER[f1] < FORDER[f2] else (f2, f1)
    return None


def rest_proximity(name: str, finger: str, hand: str) -> float:
    """0..2+ : Chebyshev distance of key from finger's rest (or natural pos)."""
    rest = REST.get((hand, finger))
    if rest is None:
        natural_col = {"L": {"P": 1, "R": 2, "M": 3, "I": 4},
                       "R": {"I": 7, "M": 8, "R": 9, "P": 10}}[hand][finger]
        return cheb((KEYS[name].row, KEYS[name].col), (1, natural_col))
    return cheb((KEYS[name].row, KEYS[name].col),
                (KEYS[rest].row, KEYS[rest].col))


def _rest_mult(name, finger, hand, up: bool) -> float:
    d = rest_proximity(name, finger, hand)
    if d > 1:
        return 1.0
    # at-rest key: the posture is the natural arched stance -> nearly free
    if up:
        return 0.05 if d == 0 else 0.60
    return 0.10 if d == 0 else 0.70


# Scissor base costs (tiered wiki model) — module constants so robustness
# sweeps can perturb them (analysis/legacy/robustness.py).
SCISSOR_MI_HSB = 1.5    # index-middle HSB: prefer-avoid
SCISSOR_MI_FSB = 6.0    # index-middle FSB: intolerant
SCISSOR_MR_RP = 6.0     # middle-ring / ring-pinky: intolerant at any distance


def scissor_cost(name1, f1, name2, f2):
    """Cost of a bigram from the scissor model. 0 if not a scissor.

    Wiki/anatomical rule (validated on canonical QWERTY CR, DR, KO, PL):
    a scissor is the SHORTER finger UP while the adjacent LONGER finger is
    DOWN.  Per pair (outer=lateral, inner=central):
      index-middle (M outer, I inner): scissor = index UP + middle DOWN
      middle-ring  (R outer, M inner): scissor = ring   UP + middle DOWN
      ring-pinky   (P outer, R inner): scissor = pinky  UP + ring   DOWN
    Reverse motions (longer up, shorter down) are the natural wave: free.
    """
    if name1 == SPACE or name2 == SPACE:
        return 0.0
    k1, k2 = KEYS[name1], KEYS[name2]
    if k1.hand != k2.hand or k1.col == k2.col:
        return 0.0
    pair = _outer_inner(f1, f2)
    if pair is None:
        return 0.0
    outer, inner = pair
    if f1 == outer:
        outer_name, inner_name, r_out, r_in = name1, name2, k1.row, k2.row
    else:
        outer_name, inner_name, r_out, r_in = name2, name1, k2.row, k1.row
    if r_out == r_in:
        return 0.0
    # direction of the wiki-type scissor, per finger-pair length rule
    if (outer, inner) == ("M", "I"):
        is_scissor = r_in < r_out       # index (inner, shorter-ish) UP
        up_name, up_f = inner_name, inner
        dn_name, dn_f = outer_name, outer
    else:  # ("R","M") or ("P","R"): shorter = outer
        is_scissor = r_out < r_in       # ring/pinky (outer) UP
        up_name, up_f = outer_name, outer
        dn_name, dn_f = inner_name, inner
    if not is_scissor:
        return 0.0
    dist = cheb((k1.row, k1.col), (k2.row, k2.col))
    hand = k1.hand
    mult = _rest_mult(up_name, up_f, hand, up=True) * \
           _rest_mult(dn_name, dn_f, hand, up=False)
    if (outer, inner) == ("M", "I"):
        base = SCISSOR_MI_HSB if dist == 1 else SCISSOR_MI_FSB
    else:
        base = SCISSOR_MR_RP
    return base * mult
