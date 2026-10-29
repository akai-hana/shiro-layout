"""
Scorer tables built on the pattern model.

    from utils.scorer import get_tables, Corpus, SPACE, N
    BIG, TRI, BIG_F, TRI_F = get_tables()

Table shapes / encodings:
  N = 31 (30 keys in fingers.KEY_ORDER = QWERTY row-major, + space as index 30)
  BIG[i,j] / TRI[i,j,k]  = cheapest total cost (per-press effort INCLUDED)
  BIG_F[i,j] = f1*5+f2 ;  TRI_F[i,j,k] = (f1,f2,f3)   with FID P0 R1 M2 I3 T4
Per-press baseline = 1.0, space = 0.6.  The alt-licence table (gates.py)
is applied as +12 per forbidden use.  Everything else comes from utils.comfort.
"""
import json
import os

import numpy as np
from pathlib import Path

from . import comfort as cg
from . import gates as GT
from . import paths
from .ansi import KEYS

N, SPACE_ID, NK = 31, 30, 30
SPACE = "@space"
FID = {"P": 0, "R": 1, "M": 2, "I": 3, "T": 4}
SPACE_EFFORT, PRESS = 0.6, 1.0
MODEL_DIR = paths.MODEL_DIR
CONFIGS = {"A:all": ("A", "all", "theta_geo_A_all.json")}
_ACTIVE = None


def load_theta(path):
    data = json.load(open(path))
    return np.array([data.get(name, cg.THETA0[i]) for i, name in enumerate(cg.PNAMES)], float)


def set_active_model(variant, theta_path):
    global _ACTIVE
    _ACTIVE = (str(variant), Path(theta_path))


def reset_tables():
    global _T
    _T = None


def load_model(cfg_name=None):
    if cfg_name is None:
        from . import config as CFG
        cfg = CFG.ensure_applied()
        var, gates, theta_path = cfg.model_variant, "all", cfg.theta_path
    else:
        cfg_name = cfg_name or os.environ.get("SHIRO_GEO_CFG", "A:all")
        var, gates, fn = CONFIGS[cfg_name]
        theta_path = MODEL_DIR / fn
    th = load_theta(str(theta_path))
    GT.set_explicit(gates)
    return cg.Model(theta=th, disallow_full_MR=(var == "A")), gates


def build_tables(cfg_name=None):
    model, gates = load_model(cfg_name)
    tabs = model.tables()
    E = tabs["E"] + PRESS                                  # (30,4)
    big = np.full((N, N), np.inf); bigf = np.zeros((N, N), np.int8)
    tri = np.full((N, N, N), np.inf); trif = np.zeros((N, N, N, 3), np.int8)
    F = np.arange(4)
    # ---- bigrams between real keys
    A, B = np.meshgrid(np.arange(NK), np.arange(NK), indexing="ij")
    A, B = A.ravel(), B.ravel()
    blk = cg.bi_many(tabs, A, B) + PRESS * 2 - 0.0         # T2 already holds E_a+E_b ; add baseline
    for n in range(len(A)):
        keys = (KEYS[A[n]], KEYS[B[n]])
        pm = GT.penalty_mask(keys)
        c = blk[n] + pm
        # per-key neighbours gate for alternation handled via penalty_mask (neighbour = other key)
        m = np.unravel_index(np.argmin(c), c.shape)
        big[A[n], B[n]] = c[m]; bigf[A[n], B[n]] = m[0] * 5 + m[1]
    # ---- trigrams between real keys
    A3, B3, C3 = [x.ravel() for x in np.meshgrid(*(np.arange(NK),) * 3, indexing="ij")]
    CH = 3000
    for s in range(0, len(A3), CH):
        sl = slice(s, s + CH)
        blk3 = cg.tri_many(model, tabs, A3[sl], B3[sl], C3[sl]) + 3 * PRESS
        for n in range(blk3.shape[0]):
            a, b, c_ = A3[sl][n], B3[sl][n], C3[sl][n]
            cc = blk3[n] + GT.penalty_mask((KEYS[a], KEYS[b], KEYS[c_]))
            m = np.unravel_index(np.argmin(cc), cc.shape)
            tri[a, b, c_] = cc[m]; trif[a, b, c_] = m
    # ---- space (thumb): effort + W_ALT, no geometry
    Emin = None
    for a in range(NK):
        ea = tabs["E"][a] + PRESS
        for sp in (SPACE_ID,):
            # key then space / space then key
            f = int(np.argmin(ea))
            big[a, sp] = ea[f] + SPACE_EFFORT + cg.W_ALT; bigf[a, sp] = f * 5 + 4
            big[sp, a] = ea[f] + SPACE_EFFORT + cg.W_ALT; bigf[sp, a] = 4 * 5 + f
    big[SPACE_ID, SPACE_ID] = 0.0; bigf[SPACE_ID, SPACE_ID] = 4 * 5 + 4
    for i in range(N):
        for j in range(N):
            for k in range(N):
                if SPACE_ID not in (i, j, k):
                    continue
                ids = (i, j, k); best = None
                import itertools
                opts = [[4] if x == SPACE_ID else [f for f in range(4) if np.isfinite(tabs["E"][x][f])] for x in ids]
                for fs in itertools.product(*opts):
                    if (i == j and fs[0] != fs[1]) or (j == k and fs[1] != fs[2]) or (i == k and fs[0] != fs[2]):
                        continue
                    c = sum((SPACE_EFFORT if x == SPACE_ID else tabs["E"][x][f] + PRESS) for x, f in zip(ids, fs))
                    # bigram terms with space are W_ALT; real-real adjacent keys use the pair table
                    for (p, q) in ((0, 1), (1, 2)):
                        if SPACE_ID in (ids[p], ids[q]):
                            c += cg.W_ALT
                        else:
                            c += tabs["PT"][ids[p], fs[p], ids[q], fs[q]]
                    if best is None or c < best[0]:
                        best = (c, fs)
                tri[i, j, k] = best[0]; trif[i, j, k] = best[1]
    return big, tri, bigf, trif


_T = None


def get_tables():
    global _T
    from . import config as CFG
    CFG.ensure_applied()
    if _T is None:
        _T = build_tables()
    return _T

from .corpus import Corpus
