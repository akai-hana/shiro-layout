"""
gates.py -- alt-finger context gates as data, plus a vectorised scanner
that finds every key bigram / trigram whose cheapest fingering in the pattern
model breaks a gate.  Gates are *not* part of the model; they are the
specification the model has to reproduce.

Semantics: a non-default finger on a key is
'blocked' in a sequence when
   mode 'never'  : always
   mode 'block'  : any OTHER key of the sequence is in the set
   mode 'only'   : NO other key of the sequence is in the set
"""
import numpy as np
from . import comfort as cg
from .fingers import KI, KEYS, NK, NF, FINGERS, FI

GATES_ALL = {
    ("X", "M"): ("only", {"C", "A"}),     # USER 2026-10-27: X->A = middle->ring is wanted
    ("W", "M"): ("only", {"Q", "X", "S"}),  # USER 2026-10-27: X->W and S->W = index->middle is wanted
    ("L", "I"): ("only", {"O"}),
    ("J", "M"): ("block", {"T", "I", "G"}),
    ("M", "M"): ("block", {"J"}),
    ("C", "M"): ("block", {"R"}),
    ("G", "M"): ("never", None),
    ("V", "M"): ("block", {"G"}),
}
# the two gates that contradict legacy rulings (X->W IM, S->W IM, fxw, wxf ...)
LEGACY_CONFLICT_GATES = set()      # no remaining conflict after the 2026-10-27 answers


# gates the pattern model is asked to reproduce from geometry alone ('geo'), and the
# residual ones kept as an explicit context table ('explicit') because they depend on
# keys the finger cannot physically interact with (J middle vs T / I / G)
EXPLICIT = {("J", "M")}
EXPLICIT_MODE = "all"     # which gate table is enforced as +XPEN context penalties: all | no_WX | none


def set_explicit(mode):
    global EXPLICIT_MODE
    EXPLICIT_MODE = mode
XPEN = 12.0                      # context-gate penalty


def gate_subset(which="all"):
    if which == "geo":
        return {k: v for k, v in GATES_ALL.items() if k not in EXPLICIT}
    if which == "geo_no_WX":
        return {k: v for k, v in GATES_ALL.items() if k not in EXPLICIT | LEGACY_CONFLICT_GATES}
    if which == "explicit":
        return gate_subset(EXPLICIT_MODE)
    if which == "all":
        return dict(GATES_ALL)
    if which == "no_WX":
        return {k: v for k, v in GATES_ALL.items() if k not in LEGACY_CONFLICT_GATES}
    if which == "none":
        return {}
    raise ValueError(which)


def forbidden_pairs(keys, gates):
    """set of (pos, finger_letter) blocked in this key sequence."""
    out = set()
    for pos, k in enumerate(keys):
        nb = [x for i, x in enumerate(keys) if i != pos]
        for (gk, gf), (mode, s) in gates.items():
            if gk != k:
                continue
            if mode == "never" or (mode == "block" and any(x in s for x in nb)) \
               or (mode == "only" and not any(x in s for x in nb)):
                out.add((pos, gf))
    return out


def _forb_arrays(n, gates):
    """forb[pos] -> bool array (NK,)*n + (NF,) : finger forbidden at pos."""
    shape = (NK,) * n
    forb = [np.zeros(shape + (NF,), bool) for _ in range(n)]
    idx = np.indices(shape)
    for pos in range(n):
        for (gk, gf), (mode, s) in gates.items():
            here = idx[pos] == KI[gk]
            nbmask = [np.isin(idx[i], [KI[x] for x in s]) if s else np.zeros(shape, bool)
                      for i in range(n) if i != pos]
            anynb = np.zeros(shape, bool)
            for m in nbmask:
                anynb |= m
            if mode == "never":
                cond = here
            elif mode == "block":
                cond = here & anynb
            else:
                cond = here & ~anynb
            forb[pos][..., FI[gf]] |= cond
    return forb


def scan(model, tabs, gates, margin=0.05, trigrams=True):
    """-> list of violations (keys, forb, best_allowed, best_forb, best_str)."""
    viol = []
    # ---------------- bigrams
    T2 = np.transpose(tabs["T2"], (0, 2, 1, 3)).copy()          # a,b,fa,fb
    ar = np.arange(NK)
    rep = ar[:, None] == ar[None, :]
    fne = np.arange(NF)[:, None] != np.arange(NF)[None, :]
    T2 = np.where(rep[:, :, None, None] & fne[None, None], np.inf, T2)
    fb2 = _forb_arrays(2, gates)
    Fm = fb2[0][:, :, :, None] | fb2[1][:, :, None, :]
    viol += _collect(T2, Fm, 2, margin, gates)
    if trigrams:
        T3 = np.empty((NK, NK, NK, NF, NF, NF))
        for a in range(NK):
            T3[a] = np.stack([np.stack([cg.tri_block(model, tabs, a, b, c) for c in range(NK)]) for b in range(NK)])
        fb3 = _forb_arrays(3, gates)
        Fm3 = (fb3[0][..., :, None, None] | fb3[1][..., None, :, None] | fb3[2][..., None, None, :])
        viol += _collect(T3, Fm3, 3, margin, gates)
    return viol


def _collect(T, Fm, n, margin, gates):
    big = 1e9
    T = np.where(np.isfinite(T), T, big)
    flat = T.reshape(T.shape[:n] + (-1,))
    Ff = Fm.reshape(flat.shape)
    best_all = flat.min(-1)
    best_forb = np.where(Ff, flat, big).min(-1)
    best_ok = np.where(~Ff, flat, big).min(-1)
    gap = best_ok - best_forb                        # <0 : legal wins
    bad = (best_forb < big / 2) & (best_ok < big / 2) & (gap > -margin)
    out = []
    for idx in zip(*np.nonzero(bad)):
        keys = tuple(KEYS[i] for i in idx)
        am = flat[idx].argmin()
        fs = np.unravel_index(am, (NF,) * n)
        out.append((keys, forbidden_pairs(keys, gates), float(best_ok[idx]),
                    float(best_forb[idx]), "".join(FINGERS[f] for f in fs)))
    return out


_PM = {}


def penalty_mask(keys, gates_name="explicit"):
    """(NF,)*n array: XPEN where a fingering uses an explicitly-gated finger/key."""
    key = (tuple(keys), gates_name, EXPLICIT_MODE)
    if key not in _PM:
        n = len(keys)
        forb = forbidden_pairs(keys, gate_subset(gates_name))
        idx = np.indices((NF,) * n)
        hit = np.zeros((NF,) * n, bool)
        for pos, f in forb:
            hit |= idx[pos] == FI[f]
        _PM[key] = np.where(hit, XPEN, 0.0)
    return _PM[key]
