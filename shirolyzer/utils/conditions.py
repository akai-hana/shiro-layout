"""Structured geometric conditions and their scoring mask.

Conditions are evaluated in list order; the first matching entry decides whether
a fingered key pair is allowed.  The shipped defaults describe the calibrated
model's native geometry rules and are left to that model, so enabling the
engine is reserved for user-edited condition lists.
"""
from __future__ import annotations

import numpy as np

from . import comfort as cg
from .ansi import H, KEYS
from .fingers import FINGERS, FI, KI, NF, NK

PENALTY = 12.0
_ENABLED = False
_PAIR_DENIED = None
_CONDITIONS = []


def _finger_set(entries) -> set[tuple[int, int]] | None:
    if not entries:
        return None
    out = set()
    for token in entries:
        finger = token[-1]
        if len(token) == 1:
            out.add((0, FI[finger]))
            out.add((1, FI[finger]))
        else:
            out.add((0 if token[0] == "L" else 1, FI[finger]))
    return out


def _matches_fingers(cond, a, fa, b, fb) -> bool:
    fingers = cond.get("_fingers")
    if fingers is None:
        return True
    return (int(H[a]), fa) in fingers or (int(H[b]), fb) in fingers


def _allowed_pair(a, fa, b, fb, conditions) -> bool:
    if H[a] != H[b] or fa == fb:
        return True
    g = float(cg.PF["g"][a, fa, b, fb])
    s = float(cg.PF["s"][a, fa, b, fb])
    drow = float(cg.PF["drow"][a, fa, b, fb])
    nadj = int(cg.PF["nadj"][a, fa, b, fb])
    rest_sep = float(cg.PF["rest_sep"][a, fa, b, fb])
    for cond in conditions:
        kind = cond["type"]
        if not _matches_fingers(cond, a, fa, b, fb):
            continue
        match = False
        if kind == "scissors":
            if cond.get("distance") == "full":
                match = abs(drow) >= 2
            else:
                match = abs(drow) == 1
            if cond.get("reverse", False):
                match = match and g < 0
        elif kind == "spread":
            match = nadj == 1 and s > rest_sep + float(cond.get("max_extra", 2.0))
        elif kind == "separation":
            match = s < float(cond.get("min_width", 1.9))
        elif kind == "crossing":
            match = nadj >= 2 and s < float(cond.get("deficit", 0.30)) + 1.0
        if match:
            return bool(cond["allow"])
    return True


def configure(conditions, enabled=False):
    global _ENABLED, _PAIR_DENIED, _CONDITIONS
    _ENABLED = bool(enabled)
    _CONDITIONS = [dict(cond, _fingers=_finger_set(cond.get("fingers"))) for cond in conditions]
    if not _ENABLED:
        _PAIR_DENIED = None
        return
    denied = np.zeros((NK, NF, NK, NF), dtype=bool)
    for a in range(NK):
        for b in range(NK):
            for fa in range(NF):
                for fb in range(NF):
                    if not cg.REACH[a, fa] or not cg.REACH[b, fb]:
                        continue
                    denied[a, fa, b, fb] = not _allowed_pair(a, fa, b, fb, _CONDITIONS)
    _PAIR_DENIED = denied


def enabled() -> bool:
    return _ENABLED


def penalty_mask(keys):
    """Penalty array for a key sequence (NF,)*n, based on adjacent pairs."""
    n = len(keys)
    if not _ENABLED or n < 2:
        return np.zeros((NF,) * n)
    ids = [KI[k] for k in keys]
    out = np.zeros((NF,) * n)
    for pos in range(n - 1):
        a, b = ids[pos], ids[pos + 1]
        block = _PAIR_DENIED[a, :, b, :]
        shape = [1] * n
        shape[pos] = NF
        shape[pos + 1] = NF
        out += np.where(block.reshape(shape), PENALTY, 0.0)
    return out
