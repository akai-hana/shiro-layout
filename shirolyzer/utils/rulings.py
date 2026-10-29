"""
rulings.py -- the user's stated comfort judgements, as data.

Every entry cites where it came from (analysis/_rebuild_notes.md section, or a
comment in fingers.py / comfort.py).  Keys are QWERTY *position* names; a
fingering string lists the finger on each key in order (P R M I).

Kinds
  top1     : this fingering must be the cheapest for this key sequence
  order    : chain of (keys, fingering) items; each must be cheaper than the next
  tie      : two fingered items must cost about the same
  infeas   : this fingered pair must be impossible (cost >= ~8, i.e. worse than
             any same-finger bigram, so any typeable alternative beats it)
  feasible : this fingered pair must NOT be impossible
  tiers    : M/I comfort ladder (list of tiers, each a list of items; tier i
             must be cheaper than tier i+1; items inside a tier roughly equal)
"""
from dataclasses import dataclass, field


@dataclass
class R:
    id: str
    kind: str
    data: object
    src: str
    family: str = ""          # mirrored twins share a family (kept in the same CV fold)
    note: str = ""
    soft: bool = False        # interpretive / low-confidence -> half weight


def bi(k1, k2, fing):
    return ("bi", (k1, k2), fing)


def tri(k1, k2, k3, fing):
    return ("tri", (k1, k2, k3), fing)


RULINGS = []


def add(*a, **k):
    RULINGS.append(R(*a, **k))


# ------------------------------------------------------------- top-1 bigrams
_T1 = [
    # family, [(k1,k2,fing), ...], source
    ("OL",  [("O", "L", "MI"), ("L", "O", "IM")],
     "v2.6 #5-6: O->L: MI is the PREFERRED default; MR only as SFB-avoiding fallback; reversals use the opposite fingers"),
    ("XQ",  [("X", "Q", "IM"), ("Q", "X", "MI")], "v2.5 #2: X->Q = index-middle"),
    ("SW",  [("S", "W", "IM"), ("W", "S", "MI")], "v2.2 #1: S->W = I->M"),
    ("XS",  [("X", "S", "IM"), ("S", "X", "MI")], "v2.2 #1: X->S = I->M"),
    ("XW",  [("X", "W", "IM"), ("W", "X", "MI")], "v2.3 #1 / v2.7 #1: X->W = index on X, middle on W"),
    ("O,",  [("O", ",", "MI"), (",", "O", "IM")], "v2.2 #1 / v2.3 #3: O->, = M->I (ring-on-O + middle-on-comma is 'ridiculous')"),
    ("L,",  [("L", ",", "MI"), (",", "L", "IM")], "v2.2 #1: L->, = M->I"),
    ("SQ",  [("S", "Q", "IM"), ("Q", "S", "MI")], "v2.2 #2: Q<->S isolated prefers index+middle"),
    ("WQ",  [("W", "Q", "MR"), ("Q", "W", "RM")], "v2.3 #4: W->Q middle->ring; pinky 'really bad'"),
    ("O.",  [("O", ".", "RP"), (".", "O", "PR")], "v2.3 #2: O<->. ring-pinky 'somewhat works'; middle-index is wrong there"),
    ("FX",  [("F", "X", "IR"), ("X", "F", "RI")], "v2.6 hard check: F->X ring-index (wide X-F gap is no smooth M roll)"),
    ("UM",  [("U", "M", "MI"), ("M", "U", "IM")], "v2.6 #1: U->M = MI ('middle extends for the U, index presses the M')"),
    ("RV",  [("R", "V", "MI"), ("V", "R", "IM")], "v2.6 #1: R->V = MI"),
    ("IM",  [("I", "M", "MI"), ("M", "I", "IM")], "v2.6 #1: I->M = MI (comfortable); v2.7 #1 easiest"),
    ("WA",  [("W", "A", "RP"), ("A", "W", "PR")], "v2.6 #4: ring<->pinky"),
]
for fam, items, src in _T1:
    for (a, b, f) in items:
        add(f"bi_{a}{b}", "top1", bi(a, b, f), src, family=fam)

# the MI-pair on O->L is the one the user called 'preferred, not a tie-break'
# (pairs below are included in the 'OL' family above)

# ------------------------------------------------------------ top-1 trigrams
add("tri_WXF", "top1", tri("W", "X", "F", "MII"),
    "v2.5 #4: wxf = MII (SFB X->F accepted over RMI full scissor)", family="FXW")
add("tri_QFB", "top1", tri("Q", "F", "B", "PMI"),
    "v2.2 #4: natural QFB = pinky-middle-index (hand tilt)", family="QFB", soft=True,
    note="interpretive: user called it 'unergonomic overall'")

# ------------------------------------------------------------------ ordering
def item(k1, f1, k2, f2):
    return (k1, f1, k2, f2)


add("ord_IM_XW_UM", "order",
    [item("I", "M", "M", "I"), item("X", "I", "W", "M"), item("U", "M", "M", "I")],
    "v2.7 #1: required order I-M < X-W < U-M", family="IMXWUM")
add("ord_RP_dir", "order", [item("W", "R", "A", "P"), item("A", "P", "W", "R")],
    "v2.6 #4: ring->pinky 'significantly' better than pinky->ring", family="RPDIR",
    note="margin 0.15 enforced")
add("ord_RM_dir", "order", [item("S", "R", "D", "M"), item("D", "M", "S", "R")],
    "v2.6 #3: ring->middle slightly better than middle->ring", family="RMDIR")
add("ord_leap", "order",
    [item("D", "M", "F", "I"), item("S", "R", "F", "I"), item("A", "P", "F", "I")],
    "comfort.py leap comment (user Q4): 'he' (pinky->index, gap 3) uncomfortable; 'st' (ring->index, gap 2) mild; adjacent roll best",
    family="LEAP")
add("tie_MI_sym", "tie", [item("D", "M", "F", "I"), item("F", "I", "D", "M")],
    "v2.6 #2: M<->I rolls equally comfortable in both directions", family="MISYM")

# ----------------------------------------------------------------- M/I ladder
# left hand, middle on E/D/C (rows 0/1/2) and index on R/F/V (rows 0/1/2)
_MK = {0: "E", 1: "D", 2: "C"}
_IK = {0: "R", 1: "F", 2: "V"}
_TIERS = [[(0, 1)], [(1, 1), (0, 0)], [(1, 2)], [(2, 2), (0, 2)], [(1, 0), (2, 1)], [(2, 0)]]
_tiers = []
for t in _TIERS:
    items = []
    for (mr, ir) in t:
        items.append(item(_MK[mr], "M", _IK[ir], "I"))
        items.append(item(_IK[ir], "I", _MK[mr], "M"))
    _tiers.append(items)
add("ladder", "tiers", _tiers,
    "comfort.py _MI_LADDER: the user's dictated ordinal ladder for left-hand M/I rolls", family="LADDER")

# ---------------------------------------------------------------- feasibility
_INF = [
    ("L", "M", ",", "R", "v2.2 #1: L->, with middle in ring's column / ring in middle's: impossible and absurd"),
    ("O", "M", ",", "R", "v2.2 #1: O->, swapped columns: impossible"),
    ("S", "R", "W", "M", "v2.2 #1: S->W stack against hand angle AND stagger: impossible"),
    ("S", "M", "W", "R", "v2.5 #1: half-span ring-up/middle-down stack stays impossible"),
    ("X", "R", "S", "M", "v2.2 #1: X->S stack: impossible"),
    ("X", "M", "S", "R", "v2.5 #1: half-span ring-up/middle-down stack stays impossible"),
    ("X", "I", "F", "M", "v2.3 (wxf): index on X + middle on F crosses 1.5 keys: impossible"),
    ("X", "R", "Q", "P", "v2.7 #4-5: ring-pinky reverse full scissor: impossible"),
    ("Q", "P", "X", "R", "v2.7 #4-5: ring-pinky reverse full scissor: impossible"),
    ("X", "I", "W", "R", "v2.3 #1: index+ring same column squeezes middle: impossible"),
]
for i, (a, fa, b, fb, src) in enumerate(_INF):
    add(f"inf_{a}{fa}{b}{fb}", "infeas", item(a, fa, b, fb), src, family=f"INF{i}")
add("inf_ORcM", "infeas", item("O", "R", ",", "M"),
    "user, this session: 'ring on O + middle on comma' = full reverse M/R scissor: disallow",
    family="INFNEW", note="only enforced in the 'disallow full M/R' variant")

_FEAS = [
    ("O", "M", "L", "R", "v2.5 #1: O->L stack is viable when stagger-aligned and natural direction"),
    ("U", "M", "M", "I", "v2.6 #1: U->M MI (crossed .75 but diagonal) is comfortable"),
    ("I", "M", "K", "I", "comfort.py CROSS_DEADZONE comment (user Q3): nc (right, M top -> I home, crossed .25) completely comfortable"),
    ("W", "R", "A", "P", "v2.6: ring-pinky half roll fine"),
    ("O", "R", ".", "P", "v2.3 #2: ring->pinky across 1 row 'somewhat works'"),
]
for i, (a, fa, b, fb, src) in enumerate(_FEAS):
    add(f"feas_{a}{fa}{b}{fb}", "feasible", item(a, fa, b, fb), src, family=f"FEAS{i}")

# ------------------------------------------------- isolated-key default fingers
# README rules A.1-A.10 with the user's corrections (Q = ring, period = ring).
_DEF = {"Q":"R","W":"R","E":"M","R":"M","T":"M","Y":"M","U":"M","I":"M","O":"R","P":"R",
        "A":"P","S":"R","D":"M","F":"I","G":"I","H":"I","J":"I","K":"M","L":"R",";":"P",
        "Z":"P","X":"R","C":"I","V":"I","B":"I","N":"I","M":"I",",":"R",".":"R","/":"P"}
# README defaults the shipped code's own isolated-key costs do NOT follow (R,T,Y -> index by
# travel cost; X -> middle; period -> pinky): not rulings in practice, so not enforced.
_NOT_HONOURED = set()           # 2026-10-28: user gave the full default fingermap; every key enforced
# U: the user's table gives index-on-U a +1.0 per-key premium (A.2 'YUI middle'); with the
# declared travel constants geometry alone puts the index on U.  Only the per-key premium
# keeps U on the middle finger, so it is NOT a pattern and not enforced in isolation (the
# U-context rulings -- U->M, I-U-J -- are enforced and pass).
# (U is enforced again after the 2026-10-27 answer: index-on-U not wanted)
for _k, _f in _DEF.items():
    if _k in _NOT_HONOURED:
        continue
    add(f"key_{_k}", "key1", (_k, _f),
        "README A.1-A.10 default finger (user corrections: Q ring, period ring)",
        family="DEFAULT_" + _k, soft=True)


# =============================================================== ROUND 8 (2026-10-26)
# Source: hard probes and the configured alt-context gates.
# (REPORT_round8.md section 3).  Keys are physical QWERTY positions.
#   cons : the fingerings satisfying (req / forbid) must beat all others by a margin
#          data = dict(keys, req=((pos,"fingers"),..), forbid=((pos,finger),..), margin)
#   band : a fingered pair's cost must lie in [lo, hi]
def cons(id, keys, req=(), forbid=(), margin=0.05, src="", family="", soft=False):
    add(id, "cons", dict(keys=tuple(keys), req=tuple(req), forbid=tuple(forbid), margin=margin),
        src, family=family or id, soft=soft)


add("r8_KJM", "top1", tri("K", "J", "M", "RMI"),
    "round8 audit: K-J-M = ring/middle/index ('hey'; K ring avoids the K-M half-SFB)", family="R8_KJM")
add("r8_FJM", "top1", tri("F", "J", "M", "IMI"),
    "round8 audit: F-J-M = index/middle/index ('ney')", family="R8_FJM")
cons("r8_MJI", "MJI", req=((0, "I"), (1, "I")), src="round8: M-J-I first two strokes index (ruled trigram beats the direct M-J bigram)")
cons("r8_HTJ", "HTJ", forbid=((2, "M"),), src="round8: H-T-J ('exp') blocks J middle")
cons("r8_IUJ", "IUJ", forbid=((2, "M"),), src="round8: I-U-J ('age') blocks J middle")
cons("r8_VGJ", "VGJ", req=((0, "I"), (1, "I")), src="round8: V-G-J ('ble') forces V and G index")
cons("r8_COR", "COR", req=((0, "I"), (2, "I")), src="round8: C-O-R ('vid') endpoints index")
cons("r8_ROC", "ROC", req=((0, "I"), (2, "I")), src="round8: R-O-C ('div') endpoints index")
cons("r8_LO", "LO", req=((0, "I"),), margin=-0.2, src="round8: L->O keeps L on index within 0.2 of the best")
add("r8_CRrev_a", "band", (("C", "I", "R", "M"), 3.0, 7.0),
    "round8 W_CR_REVERSE=3.6: index/middle FULL-SPAN reverse scissor is 'barely possible, very uncomfortable'",
    family="R8_CRREV")
add("r8_CRrev_b", "band", (("R", "M", "C", "I"), 3.0, 7.0),
    "round8 W_CR_REVERSE=3.6 (other direction)", family="R8_CRREV")

# Legacy rulings that CONTRADICT the round-8 gates 'W middle only with Q' / 'X middle only with C'.
# They are switched off when those two gates are on (and back on, with the two gates off,
# in the 'legacy' configuration).  Quantified in REPORT.
SUPERSEDED_BY_WX_GATES = set()


# =============================================================== USER ANSWERS 2026-10-27
add("r9_WD_RM", "top1", bi("W", "D", "RM"),
    "user 2026-10-27: no strain felt on W->D with ring on W / middle on D (th)", family="R9_WD")
add("r9_DW_MR", "top1", bi("D", "W", "MR"),
    "user 2026-10-27: middle->ring (D->W) is the more comfortable direction on W-D", family="R9_WD")
add("r9_ord_WD", "order", [item("D", "M", "W", "R"), item("W", "R", "D", "M")],
    "user 2026-10-27: middle->ring on W-D is more comfortable than ring->middle", family="R9_WD")
add("r9_inf_XRAP", "infeas", item("X", "R", "A", "P"),
    "user 2026-10-27: ring on X + pinky on A is practically impossible", family="R9_XA")
add("r9_inf_APXR", "infeas", item("A", "P", "X", "R"),
    "user 2026-10-27: ring on X + pinky on A is practically impossible", family="R9_XA")
for _a, _b, _f in (("U", "J", "MI"), ("J", "U", "IM"), ("U", "H", "MI"), ("H", "U", "IM")):
    add(f"r9_{_a}{_b}", "top1", bi(_a, _b, _f),
        "user 2026-10-27: UJ/UH: middle on U, index on the closer key (no index on U)", family="R9_U")


# Rulings that the 2026-10-27 answers contradict and that are waiting for the user's call.
# They are NOT enforced by default (set SHIRO_KEEP_UNRESOLVED=1 to enforce them and see the failures).
#  SQ : v2.2 #2 "Q<->S isolated prefers index+middle" vs 2026-10-27 "AS/SA: middle+ring is more comfortable"
#       and "middle->ring on W-D is more comfortable" -- the model picks middle on S / ring on Q (0.58 vs IM 1.03).
UNRESOLVED_FAMILIES = {"R13_JY"}   # J-Y/Y-J "right-index/left-middle": same-hand crossing vs other hand? asked the user (2026-10-29)


# =============================================================== USER ANSWERS 2026-10-27 (second batch)
add("r9_XA_IR", "top1", bi("X", "A", "IR"),
    "user 2026-10-27: X->A: index->ring is preferred over any other combination", family="R9_XA2")
add("r9_AX_RI", "top1", bi("A", "X", "RI"),
    "user 2026-10-27: X-A: index+ring most preferred (A->X ring->index)", family="R9_XA2")
add("r9_ord_XA", "order", [item("X", "I", "A", "R"), item("X", "M", "A", "R")],
    "user 2026-10-27: X->A: index+ring first, middle+ring second", family="R9_XA3")
add("r9_OK_RM", "top1", bi("O", "K", "RM"), "user 2026-10-27: O->K ring->middle", family="R9_OK")
add("r9_KO_MR", "top1", bi("K", "O", "MR"), "user 2026-10-27: K->O middle->ring (mirror of O->K)", family="R9_OK", soft=True)
add("r9_K;_MP", "top1", bi("K", ";", "MP"), "user 2026-10-27: K->; middle->pinky", family="R9_KS")
add("r9_;K_PM", "top1", bi(";", "K", "PM"), "user 2026-10-27: ;->K pinky->middle", family="R9_KS")
add("r9_AE_PM", "top1", bi("A", "E", "PM"), "user 2026-10-27: A->E pinky->middle", family="R9_AE")
add("r9_EA_MP", "top1", bi("E", "A", "MP"), "user 2026-10-27: E->A middle->pinky (mirror)", family="R9_AE", soft=True)

# 'Defaults first': a bigram whose two keys are both REST keys (A W E F / ; O I J) must use the rest fingers
_REST = {0: [("A", "P"), ("W", "R"), ("E", "M"), ("F", "I")], 1: [(";", "P"), ("O", "R"), ("I", "M"), ("J", "I")]}
for _h, _lst in _REST.items():
    for (_a, _fa) in _lst:
        for (_b, _fb) in _lst:
            if _a != _b:
                add(f"rest_{_a}{_b}", "top1", bi(_a, _b, _fa + _fb),
                    "user 2026-10-27: both keys at rest -> the rest fingers (zero travel); defaults first",
                    family="REST_" + "".join(sorted(_a + _b)))


# =============================================================== USER ANSWERS 2026-10-28
add("r10_inf_XIAP", "infeas", item("X", "I", "A", "P"),
    "user 2026-10-28: index + pinky with only one column of space is madness (X index, A pinky)", family="R10_XA")
add("r10_inf_APXI", "infeas", item("A", "P", "X", "I"),
    "user 2026-10-28: index + pinky with only one column of space is madness (A pinky, X index)", family="R10_XA")
add("r10_ord_XA_IM", "order", [item("X", "I", "A", "R"), item("X", "I", "A", "M")],
    "user 2026-10-28: X->A index->middle is wanted (rolls easily with index on X); index->ring stays the favourite", family="R10_XA2")
add("r10_XA_IM_top2", "order", [item("X", "I", "A", "M"), item("X", "M", "A", "P")],
    "user 2026-10-28 pm: X->A middle + pinky on A is out (middle on bottom row is bad; pinky on A = reverse scissor)", family="R10_XA2")
add("r10_XA_MR_top2", "order", [item("X", "M", "A", "R"), item("X", "M", "A", "P")],
    "user 2026-10-28 pm: X->A middle + pinky on A is out (middle on bottom row is bad; pinky on A = reverse scissor)", family="R10_XA2")
add("r10_TG_MI", "top1", bi("T", "G", "MI"), "user 2026-10-28: TG = left middle / left index (T is a middle-finger key)", family="R10_TG")
add("r10_GT_IM", "top1", bi("G", "T", "IM"), "user 2026-10-28: GT mirror of TG", family="R10_TG")


# =============================================================== USER ANSWERS 2026-10-28 (afternoon)
add("r11_ord_XA_MR_IM", "order", [item("X", "M", "A", "R"), item("X", "I", "A", "M")],
    "user 2026-10-28 pm: X->A prefers middle->ring over index->middle", family="R11_XA")
add("r11_DA_MP", "top1", bi("D", "A", "MP"), "user 2026-10-28 pm: D->A middle->pinky (middle travels one key)", family="R11_DA")
add("r11_AD_PM", "top1", bi("A", "D", "PM"), "user 2026-10-28 pm: A->D pinky->middle", family="R11_DA")
add("r11_ord_DA", "order", [item("D", "M", "A", "P"), item("D", "I", "A", "P")],
    "user 2026-10-28 pm: D-A index instead of middle is awkward", family="R11_DA")
add("r11_FS_IR", "top1", bi("F", "S", "IR"), "user 2026-10-28 pm: F->S index->ring (rest-key rule)", family="R11_FS")
add("r11_SF_RI", "top1", bi("S", "F", "RI"), "user 2026-10-28 pm: S->F ring->index", family="R11_FS")
add("r11_OJ_RI", "top1", bi("O", "J", "RI"), "user 2026-10-28: O->J ring->index (both at rest)", family="R11_OJ")
add("r11_JO_IR", "top1", bi("J", "O", "IR"), "user 2026-10-28: J->O index->ring (both at rest)", family="R11_OJ")
add("r11_RT_RM", "top1", bi("R", "T", "RM"), "user 2026-10-28 pm: RT = ring on R, middle on T", family="R11_RT")
add("r11_TR_MR", "top1", bi("T", "R", "MR"), "user 2026-10-28 pm: TR = middle on T, ring on R", family="R11_RT")

add("r12_YU_MR", "top1", bi("Y", "U", "MR"), "user 2026-10-28 late: YU and RT/TR both ring-middle / middle-ring (Y middle, U ring)", family="R12_YU")
add("r12_UY_RM", "top1", bi("U", "Y", "RM"), "user 2026-10-28 late: UY mirror (U ring, Y middle)", family="R12_YU")


# =============================================================== USER ANSWERS 2026-10-29
add("r13_AS_PR", "top1", bi("A", "S", "PR"), "user 2026-10-29 (corrected): A-S pinky->ring (supersedes the 2026-10-27 ring/middle answer)", family="R13_AS")
add("r13_SA_RP", "top1", bi("S", "A", "RP"), "user 2026-10-29 (corrected): S-A mirror of A-S (ring->pinky)", family="R13_AS", soft=True)
add("r13_;U_PM", "top1", bi(";", "U", "PM"), "user 2026-10-29: ;-U pinky->middle", family="R13_SU")
add("r13_U;_MP", "top1", bi("U", ";", "MP"), "user 2026-10-29: U-; mirror (middle->pinky)", family="R13_SU", soft=True)
add("r13_band_;RUM", "band", (item(";", "R", "U", "M"), 5.0, 12.0),
    "user 2026-10-29: ring-middle on ;-U 'makes absolutely no sense and is borderline impossible'", family="R13_SU2")
add("r13_band_URM;", "band", (item("U", "M", ";", "R"), 5.0, 12.0),
    "user 2026-10-29: ring-middle on ;-U (reverse order) borderline impossible", family="R13_SU2")
add("r13_JY_IM", "top1", bi("J", "Y", "IM"), "user 2026-10-29: J-Y index on J, middle on Y (both default fingers)", family="R13_JY")
add("r13_YJ_MI", "top1", bi("Y", "J", "MI"), "user 2026-10-29: Y-J middle on Y, index on J", family="R13_JY")
add("r13_TF_IM", "top1", bi("T", "F", "IM"), "user 2026-10-29: T-F index on T, middle on F", family="R13_TF")
add("r13_FT_MI", "top1", bi("F", "T", "MI"), "user 2026-10-29: F-T middle on F, index on T", family="R13_TF")
add("r13_;L_PR", "top1", bi(";", "L", "PR"), "user 2026-10-29: ;-L pinky->ring", family="R13_SL")
add("r13_L;_RP", "top1", bi("L", ";", "RP"), "user 2026-10-29: L-; mirror (ring->pinky)", family="R13_SL", soft=True)


def load_rulings(path):
    """Load a RULINGS list from a Python file, for calibration overrides."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("shirolyzer_rulings_override", path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load rulings from {path!r}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    if not hasattr(module, "RULINGS"):
        raise ImportError(f"{path!r} does not define RULINGS")
    return module.RULINGS
