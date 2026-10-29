"""Physical key geometry, finger constants, and default fingering.

Keys use QWERTY position names.  Fingers are P (pinky), R (ring), M (middle),
and I (index).  This module contains only geometry and declared constants; the
cost model lives in :mod:`utils.comfort`.
"""
from __future__ import annotations
import itertools
import math
from dataclasses import dataclass, field
import numpy as np

# ----------------------------------------------------------------- geometry
ROWS = ["QWERTYUIOP", "ASDFGHJKL;", "ZXCVBNM,./"]
KEYS = [k for r in ROWS for k in r]                 # 30 keys, QWERTY positions
KI = {k: i for i, k in enumerate(KEYS)}
_ROW_OFFSET = [0.0, 0.25, 0.75]                     # ANSI stagger (key widths)
_cx, _cy, _hand, _row = {}, {}, {}, {}
for r, row in enumerate(ROWS):
    for c, k in enumerate(row):
        _cx[k] = c + _ROW_OFFSET[r] + 0.5
        _cy[k] = r + 0.5
        _hand[k] = 0 if c < 5 else 1                # 0 = left, 1 = right
        _row[k] = r

FINGERS = "PRMI"                                    # index 0..3 = pinky..index
FI = {f: i for i, f in enumerate(FINGERS)}
LENGTH_RANK = np.array([0.0, 1.0, 3.0, 2.0])        # P R M I : middle longest
REST = {0: {"P": "A", "R": "W", "M": "E", "I": "F"},
        1: {"P": ";", "R": "O", "M": "I", "I": "J"}}

NK, NF = len(KEYS), 4

# README / fingers.py default finger of each key (A.1-A.10 with the user's corrections).
# 'Defaults first' (user, 2026-10-27): a non-default finger pays e_nd.  R, T, Y, X and period are
# exempt: the shipped code itself picks another finger for them in isolation.
DEFAULT_FINGER = {"Q":"R","W":"R","E":"M","R":"M","T":"M","Y":"M","U":"M","I":"M","O":"R","P":"R",
                  "A":"P","S":"R","D":"M","F":"I","G":"I","H":"I","J":"I","K":"M","L":"R",";":"P",
                  "Z":"P","X":"R","C":"I","V":"I","B":"I","N":"I","M":"I",",":"R",".":"R","/":"P"}
NODEF_EXEMPT = set()          # 2026-10-28: the user's default map is authoritative for every key
REST_KEYS = set("AWEF;OIJ")
U = np.array([(_cx[k] if _hand[k] == 0 else -_cx[k]) for k in KEYS])   # toward centre +
V = np.array([_cy[k] for k in KEYS])
H = np.array([_hand[k] for k in KEYS])
ROWI = np.array([_row[k] for k in KEYS])
CX = np.array([_cx[k] for k in KEYS])


def _rest_uv(hand, f):
    k = REST[hand][FINGERS[f]]
    return U[KI[k]], V[KI[k]]


REST_U = np.zeros((2, NF))
REST_V = np.zeros((2, NF))
for h_ in (0, 1):
    for f_ in range(NF):
        REST_U[h_, f_], REST_V[h_, f_] = _rest_uv(h_, f_)

# lateral room from finger f's rest to the next-inner finger's rest (per hand)
INNER_SEP = np.zeros((2, NF))
for _h in (0, 1):
    for _f in range(NF - 1):
        INNER_SEP[_h, _f] = REST_U[_h, _f + 1] - REST_U[_h, _f]
    INNER_SEP[_h, NF - 1] = 99.0

# displacement of key k from finger f's rest (same-hand fingers only)
DU = np.zeros((NK, NF))
DV = np.zeros((NK, NF))
for k in range(NK):
    for f in range(NF):
        DU[k, f] = U[k] - REST_U[H[k], f]
        DV[k, f] = V[k] - REST_V[H[k], f]

# Reach envelope per finger.  Instead of listing ~80 (key, finger) alternates
# by hand, the envelope is the CONVEX HULL of the (du, dv) positions the user
# approved in fingers.py (escape-only entries excluded; pinky-up-to-Q added as
# an escape that e_up_P prices).  du: toward the hand centre; dv: down.
# The hull reproduces the approved list almost exactly (a handful of harmless
# extra keys such as ring-on-D) and, unlike boxes, captures that a finger's
# inward reach shrinks as it moves down a row.
APPROVED = {
    "P": [(-0.5, 1.0), (-0.25, -1.0), (0.0, 0.0), (0.5, 1.0)],
    "R": [(-1.25, 1.0), (-1.0, 0.0), (-0.75, 2.0), (-0.25, 1.0), (0.0, 0.0), (0.25, 1.0), (0.25, 2.0), (0.75, 1.0), (0.75, 2.0), (1.0, 0.0), (2.0, 0.0)],
    "M": [(-2.0, 0.0), (-1.75, 2.0), (-1.25, 1.0), (-1.0, 0.0), (-0.75, 1.0), (-0.75, 2.0), (-0.25, 1.0), (-0.25, 2.0), (0.0, 0.0), (0.25, 1.0), (0.25, 2.0), (0.75, 1.0), (0.75, 2.0), (1.0, 0.0), (1.25, 1.0), (1.75, 2.0), (2.0, 0.0), (2.25, 1.0), (2.75, 2.0)],
    "I": [(-2.5, 1.0), (-2.0, 0.0), (-1.5, 1.0), (-1.0, 0.0), (-0.5, 1.0), (-0.25, -1.0), (0.0, 0.0), (0.25, -1.0), (0.5, 1.0), (0.75, -1.0), (1.0, 0.0), (1.25, -1.0), (1.5, 1.0)],
}
from scipy.spatial import Delaunay as _Del
REACH = np.zeros((NK, NF), dtype=bool)
for _f, _name in enumerate(FINGERS):
    _hull = _Del(np.array(APPROVED[_name]))
    REACH[:, _f] = _hull.find_simplex(np.stack([DU[:, _f], DV[:, _f]], 1), tol=1e-9) >= 0

# keys the user actually listed per (hand, finger) in fingers.py; hull extras
# beyond this list stay reachable but pay e_unl until the user blesses them
LISTED = {
    (0, "P"): "QAZ", (1, "P"): ";./",
    (0, "R"): "QWERASXD"[:0] + "QWERASXA"[:0] + "QWERSXA", (1, "R"): "UIOPKL;,.",
    (0, "M"): "QWERTSDFGXCVB", (1, "M"): "YUIOJKLM,.",
    (0, "I"): "WXRTSDFGCVB", (1, "I"): "YUHJKLNM,.",
}
UNLISTED = np.zeros((NK, NF), dtype=bool)
for _k in range(NK):
    for _f, _name in enumerate(FINGERS):
        if REACH[_k, _f] and KEYS[_k] not in LISTED[(int(H[_k]), _name)]:
            UNLISTED[_k, _f] = True

