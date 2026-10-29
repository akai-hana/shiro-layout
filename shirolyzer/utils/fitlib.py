"""Loss / evaluation of a comfort_geo.Model against rulings.RULINGS."""
import os
from pathlib import Path
import numpy as np
from . import comfort as cg
from .fingers import KI, FI, FINGERS, NF
from . import rulings as RL
from . import gates as GT

M_TOP1, M_ORD, TOL_TIE, M_TIER, TIER_WITHIN, IMP_THR = 0.05, 0.02, 0.08, 0.02, 0.15, 8.0
import os as _os
TIER_W = float(_os.environ.get("SHIRO_TIER_W", "1.5"))
FAIL_PEN = float(_os.environ.get("SHIRO_FAIL_PEN", "0.0"))
ORDER_MARGIN = {"RPDIR": 0.15}


def _t2(tabs, it):
    a, fa, b, fb = it
    return tabs["T2"][KI[a], FI[fa], KI[b], FI[fb]]


def _block(model, tabs, keys):
    ids = [KI[k] for k in keys]
    if len(ids) == 2:
        t = tabs["T2"][ids[0], :, ids[1], :].copy()
        if ids[0] == ids[1]:
            t = np.where(np.eye(NF, dtype=bool), t, np.inf)
        return t
    return cg.tri_block(model, tabs, *ids)


def _pt(tabs, it):
    a, fa, b, fb = it
    return tabs["PT"][KI[a], FI[fa], KI[b], FI[fb]]


class Cfg:
    """disallow : full-span reverse M/R is impossible (user's 2026-10 rule) vs 'priced'
       gates    : 'all' (round-8 gates incl. W/X middle, legacy W/X rulings dropped),
                  'no_WX' (legacy W/X rulings kept, those two gates dropped), 'none'"""
    def __init__(self, disallow=True, gates="all"):
        self.disallow, self.gates = disallow, gates

    def __repr__(self):
        return f"Cfg(disallow={self.disallow}, gates={self.gates})"


def active(r, cfg):
    if r.family in os.environ.get("SHIRO_DROP", "").split(","):
        return False
    if r.family in RL.UNRESOLVED_FAMILIES and not os.environ.get("SHIRO_KEEP_UNRESOLVED"):
        return False
    if isinstance(cfg, bool):
        cfg = Cfg(disallow=cfg, gates="none")
    if r.id == "inf_ORcM": return cfg.disallow
    if r.id in ("feas_XMWR", "tie_XW_MR"): return (not cfg.disallow) and cfg.gates != "all"
    if r.id == "tri_FXW": return (not cfg.disallow) and cfg.gates != "all"
    if r.id in RL.SUPERSEDED_BY_WX_GATES and cfg.gates == "all": return False
    if r.kind == "cons" and r.id.startswith("r8_") is False: return True
    return True


def eval_ruling(model, tabs, r):
    """-> (loss, ok, text)"""
    w = 0.5 if r.soft else 1.0
    if r.kind == "top1":
        typ, keys, fing = r.data
        if typ == "bi":
            a, b = KI[keys[0]], KI[keys[1]]
            pm = GT.penalty_mask(keys)
            cand = [(float(tabs["T2"][a, fa, b, fb]) + float(pm[fa, fb]), FINGERS[fa] + FINGERS[fb])
                    for fa in range(NF) for fb in range(NF)
                    if np.isfinite(tabs["T2"][a, fa, b, fb])]
        else:
            tb = cg.tri_block(model, tabs, *[KI[k] for k in keys]) + GT.penalty_mask(keys)
            cand = sorted((float(tb[x]), FINGERS[x[0]] + FINGERS[x[1]] + FINGERS[x[2]])
                          for x in np.ndindex(tb.shape) if np.isfinite(tb[x]))
        cand.sort()
        tgt = [c for c, f in cand if f == fing]
        if not tgt:
            return 5.0 * w, False, f"{'->'.join(keys)} want {fing}: not reachable"
        ct = tgt[0]
        others = [(c, f) for c, f in cand if f != fing]
        loss = sum(max(0.0, M_TOP1 + ct - c) for c, f in others[:6])
        ok = cand[0][1] == fing
        return w * loss, ok, f"{'->'.join(keys)} want {fing} got {cand[0][1]} ({cand[0][0]:.2f}; target {ct:.2f})"
    if r.kind == "key1":
        w = 1.0
        k, fing = r.data
        e = tabs["E"][KI[k]]
        ct = float(e[FI[fing]])
        others = [(float(e[i]), FINGERS[i]) for i in range(NF) if FINGERS[i] != fing and np.isfinite(e[i])]
        loss = sum(max(0.0, 0.03 + ct - c) for c, _ in others)
        best = min(others + [(ct, fing)])
        return w * loss, best[1] == fing, f"{k}: want {fing} got {best[1]} ({best[0]:.2f}; default {ct:.2f})"
    if r.kind == "order":
        costs = [float(_t2(tabs, it)) for it in r.data]
        m = ORDER_MARGIN.get(r.family, M_ORD)
        loss = sum(max(0.0, m + costs[i] - costs[i + 1]) for i in range(len(costs) - 1))
        ok = all(costs[i] + 1e-9 < costs[i + 1] for i in range(len(costs) - 1))
        return w * loss, ok, "order " + " < ".join(f"{c:.2f}" for c in costs)
    if r.kind == "tie":
        c = [float(_t2(tabs, it)) for it in r.data]
        d = abs(c[0] - c[1])
        return w * max(0.0, d - TOL_TIE), d <= TOL_TIE, f"tie {c[0]:.2f} vs {c[1]:.2f}"
    if r.kind == "infeas":
        c = float(_pt(tabs, r.data))
        return w * max(0.0, IMP_THR - c), c >= IMP_THR - 0.01, f"{r.data} pair cost {c:.2f} (need >= {IMP_THR})"
    if r.kind == "feasible":
        c = float(_pt(tabs, r.data))
        return w * max(0.0, c - 5.0), c < IMP_THR - 1.0, f"{r.data} pair cost {c:.2f} (need < 7)"
    if r.kind == "tiers":
        tiers = r.data
        tc = [[float(_t2(tabs, it)) for it in t] for t in tiers]
        loss = 0.0
        ok = True
        for i in range(len(tc) - 1):
            gap = min(tc[i + 1]) - max(tc[i])
            loss += max(0.0, M_TIER - gap)
            ok &= gap > 0
        for t in tc:
            loss += max(0.0, (max(t) - min(t)) - TIER_WITHIN)
        mx = [f"{min(t):.2f}-{max(t):.2f}" for t in tc]
        return w * loss, ok, "tiers " + " | ".join(mx)
    if r.kind == "cons":
        d = r.data
        t = _block(model, tabs, d["keys"]) + GT.penalty_mask(d["keys"])
        n = len(d["keys"])
        idx = np.indices((NF,) * n)
        m = np.ones((NF,) * n, bool)
        for pos, fs in d["req"]:
            m &= np.isin(idx[pos], [FI[f] for f in fs])
        for pos, f in d["forbid"]:
            m &= idx[pos] != FI[f]
        A = t[m]; B = t[~m]
        A = A[np.isfinite(A)]; B = np.sort(B[np.isfinite(B)])
        mg = d["margin"]
        if len(A) == 0:
            return 5.0 * w, False, f"{'-'.join(d['keys'])}: required fingering unreachable"
        ca = float(A.min())
        if len(B) == 0:
            return 0.0, True, "no alternative"
        loss = float(np.maximum(0.0, ca - B[:6] + mg).sum())
        ok = ca - float(B[0]) <= -mg + 1e-9 if mg >= 0 else ca - float(B[0]) <= -mg
        best_all = "A" if ca <= B[0] else "B"
        return w * loss, bool(ok), f"{'-'.join(d['keys'])} legal {ca:.2f} vs best-other {B[0]:.2f}"
    if r.kind == "band":
        item, lo, hi = r.data
        c = float(_pt(tabs, item))
        return w * (max(0.0, lo - c) + max(0.0, c - hi)), lo - 0.02 <= c <= hi, f"{item} pair cost {c:.2f} (want {lo}-{hi})"
    raise ValueError(r.kind)


import json as _json
_AN = None


def anchor_arrays():
    global _AN
    if _AN is None:
        rows = _json.load(open(Path(__file__).resolve().parents[1] / "config" / "model" / "anchor_pairs.json"))
        # default fingers the shipped model itself does not follow (see rulings._NOT_HONOURED)
        rows = [r for r in rows if not ({r[0], r[2]} & set("RTYX."))]
        # ring on the bottom row + pinky on the home row: impossible by the user's 2026-10-27 ruling
        rows = [r for r in rows if not ({r[1], r[3]} == {"R", "P"} and
                ((r[1] == "R" and r[0] in "X,." and r[2] in "A;") or (r[3] == "R" and r[2] in "X,." and r[0] in "A;")))]
        _AN = (np.array([KI[r[0]] for r in rows]), np.array([FI[r[1]] for r in rows]),
               np.array([KI[r[2]] for r in rows]), np.array([FI[r[3]] for r in rows]),
               np.array([r[4] for r in rows]))
    return _AN


ANCHOR_W, ANCHOR_TOL = 0.04, 0.25


def anchor_loss(tabs):
    a, fa, b, fb, c = anchor_arrays()
    d = tabs["PT"][a, fa, b, fb] - c
    d = np.where(np.isfinite(d), d, 10.0)
    ex = np.maximum(np.abs(d) - ANCHOR_TOL, 0.0)
    return ANCHOR_W * float((ex ** 2).sum()), int((ex > 0).sum()), len(c)


def total_loss(theta, cfg, subset=None, reg=0.0, return_detail=False, extra=(), gateset=None):
    if isinstance(cfg, bool): cfg = Cfg(disallow=cfg, gates="none")
    disallow = cfg.disallow
    GT.set_explicit(cfg.gates)
    model = cg.Model(theta=np.asarray(theta, float), disallow_full_MR=disallow)
    tabs = model.tables()
    loss, details = 0.0, []
    for r in RL.RULINGS:
        if not active(r, cfg):
            continue
        if subset is not None and r.family not in subset:
            continue
        l, ok, txt = eval_ruling(model, tabs, r)
        if r.kind == "tiers":
            l *= TIER_W      # the ladder is a long ordinal chain of near-ties; weight it so a miss shows up in the loss
        loss += l + (FAIL_PEN if not ok else 0.0)
        details.append((r, ok, l, txt))
    for r in extra:
        l, ok, txt = eval_ruling(model, tabs, r)
        loss += l
        details.append((r, ok, l, txt))
    if ANCHOR_W:
        al, nb_a, n_a = anchor_loss(tabs)
        loss += al
        if return_detail: details.append((type('A',(),{'id':'anchor_default_pairs','family':'ANCHOR'})(), True, al, f"{nb_a}/{n_a} default-finger pairs more than {ANCHOR_TOL} away from the shipped price"))
    if gateset is not None and len(gateset):
        gl, nb = gateset.losses(model, tabs)
        loss += gl
        if return_detail: details.append((type('G',(),{'id':'gate_instances','family':'GATES'})(), nb == 0, gl, f"{nb} of {len(gateset)} gate instances violated"))
    if reg:
        scale = 0.25 * (cg.HI - cg.LO)
        loss += reg * float(np.sum(((np.asarray(theta) - cg.THETA0) / scale) ** 2))
    return (loss, details) if return_detail else loss


if __name__ == "__main__":
    import time
    for dis in (True, False):
        t = time.time()
        l, det = total_loss(cg.THETA0, dis, return_detail=True)
        print(f"\n=== disallow_full_MR={dis}: loss {l:.3f}  ({(time.time()-t)*1000:.0f} ms)  "
              f"pass {sum(d[1] for d in det)}/{len(det)}")
        for r, ok, lo, txt in det:
            if not ok:
                print(f"  FAIL {r.id:14s} {txt}")
