"""ANSI physical geometry: stagger, hand, row, rest posture, and displacement.

Key names are QWERTY positions.  Finger names are P (pinky), R (ring),
M (middle) and I (index).  This module owns physical geometry only; finger
defaults and finger-specific heuristics live in :mod:`utils.fingers`.
"""
from __future__ import annotations

import numpy as np
from scipy.spatial import Delaunay as _Del

from ..fingers import FINGERS, FI, KEYS, KI, NK, NF

ROWS = ["QWERTYUIOP", "ASDFGHJKL;", "ZXCVBNM,./"]
ROW_OFFSET = [0.0, 0.25, 0.75]
_cx, _cy, _hand, _row = {}, {}, {}, {}
for _r, _row_name in enumerate(ROWS):
    for _c, _key in enumerate(_row_name):
        _cx[_key] = _c + ROW_OFFSET[_r] + 0.5
        _cy[_key] = _r + 0.5
        _hand[_key] = 0 if _c < 5 else 1
        _row[_key] = _r

REST = {0: {"P": "A", "R": "W", "M": "E", "I": "F"},
        1: {"P": ";", "R": "O", "M": "I", "I": "J"}}
REST_KEYS = set("AWEF;OIJ")

U = np.array([(_cx[k] if _hand[k] == 0 else -_cx[k]) for k in KEYS])
V = np.array([_cy[k] for k in KEYS])
H = np.array([_hand[k] for k in KEYS])
ROWI = np.array([_row[k] for k in KEYS])
CX = np.array([_cx[k] for k in KEYS])


def _rest_uv(hand, f):
    key = REST[hand][FINGERS[f]]
    return U[KI[key]], V[KI[key]]


REST_U = np.zeros((2, NF))
REST_V = np.zeros((2, NF))
for _h in (0, 1):
    for _f in range(NF):
        REST_U[_h, _f], REST_V[_h, _f] = _rest_uv(_h, _f)

INNER_SEP = np.zeros((2, NF))
for _h in (0, 1):
    for _f in range(NF - 1):
        INNER_SEP[_h, _f] = REST_U[_h, _f + 1] - REST_U[_h, _f]
    INNER_SEP[_h, NF - 1] = 99.0

DU = np.zeros((NK, NF))
DV = np.zeros((NK, NF))
for _k in range(NK):
    for _f in range(NF):
        DU[_k, _f] = U[_k] - REST_U[H[_k], _f]
        DV[_k, _f] = V[_k] - REST_V[H[_k], _f]

APPROVED = {
    "P": [(-0.5, 1.0), (-0.25, -1.0), (0.0, 0.0), (0.5, 1.0)],
    "R": [(-1.25, 1.0), (-1.0, 0.0), (-0.75, 2.0), (-0.25, 1.0), (0.0, 0.0), (0.25, 1.0), (0.25, 2.0), (0.75, 1.0), (0.75, 2.0), (1.0, 0.0), (2.0, 0.0)],
    "M": [(-2.0, 0.0), (-1.75, 2.0), (-1.25, 1.0), (-1.0, 0.0), (-0.75, 1.0), (-0.75, 2.0), (-0.25, 1.0), (-0.25, 2.0), (0.0, 0.0), (0.25, 1.0), (0.25, 2.0), (0.75, 1.0), (0.75, 2.0), (1.0, 0.0), (1.25, 1.0), (1.75, 2.0), (2.0, 0.0), (2.25, 1.0), (2.75, 2.0)],
    "I": [(-2.5, 1.0), (-2.0, 0.0), (-1.5, 1.0), (-1.0, 0.0), (-0.5, 1.0), (-0.25, -1.0), (0.0, 0.0), (0.25, -1.0), (0.5, 1.0), (0.75, -1.0), (1.0, 0.0), (1.25, -1.0), (1.5, 1.0)],
}
REACH = np.zeros((NK, NF), dtype=bool)
for _f, _name in enumerate(FINGERS):
    _hull = _Del(np.array(APPROVED[_name]))
    REACH[:, _f] = _hull.find_simplex(
        np.stack([DU[:, _f], DV[:, _f]], 1), tol=1e-9) >= 0

LISTED = {
    (0, "P"): "QAZ", (1, "P"): ";./",
    (0, "R"): "QWERSXA", (1, "R"): "UIOPKL;,.",
    (0, "M"): "QWERTSDFGXCVB", (1, "M"): "YUIOJKLM,.",
    (0, "I"): "WXRTSDFGCVB", (1, "I"): "YUHJKLNM,.",
}
UNLISTED = np.zeros((NK, NF), dtype=bool)
for _k in range(NK):
    for _f, _name in enumerate(FINGERS):
        if REACH[_k, _f] and KEYS[_k] not in LISTED[(int(H[_k]), _name)]:
            UNLISTED[_k, _f] = True
