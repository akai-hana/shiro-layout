"""Cutting-plane fit: model must satisfy every ruling + every round-8 gate instance."""
import sys, os, json, time, pickle

import numpy as np
from scipy.optimize import minimize, differential_evolution

from ..utils import comfort as cg
from ..utils import fitlib as FL
from ..utils import gates as G
from ..utils.gateset import GateSet

REG = 0.02


def load_theta(path):
    d = json.load(open(path))
    th = cg.THETA0.copy()
    for i, n in enumerate(cg.PNAMES):
        if n in d:
            th[i] = d[n]
    return th


def save_theta(th, path):
    json.dump({n: float(v) for n, v in zip(cg.PNAMES, th)}, open(path, "w"), indent=1)


def run(cfg, gate_set_name, theta0, rounds=12, per_round=150, out="theta", log=print, gs=None, powell_iter=3):
    gates = G.gate_subset(gate_set_name)
    gs = gs or GateSet()
    th = np.array(theta0, float)
    lo, hi = cg.LO, cg.HI
    for rd in range(rounds):
        model = cg.Model(theta=th, disallow_full_MR=cfg.disallow)
        tabs = model.tables()
        viol = G.scan(model, tabs, gates) if gates else []
        viol.sort(key=lambda v: -(v[2] - v[3]))
        added = 0
        for keys, forb, ok, fo, bs in viol:
            if gs.add(keys, forb):
                added += 1
            if added >= per_round:
                break
        l, det = FL.total_loss(th, cfg, reg=REG, return_detail=True, gateset=gs)
        fails = [d for d in det if not d[1]]
        log(f"[round {rd}] violations in scan: {len(viol)} (bi {sum(len(v[0])==2 for v in viol)}), "
            f"gate set {len(gs)} (+{added}); loss {l:.3f}; failing rulings {len(fails)}/{len(det)}")
        if not viol and not fails:
            break
        f = lambda x: FL.total_loss(np.clip(x, lo, hi), cfg, reg=REG, gateset=gs)
        for _ in range(powell_iter):
            res = minimize(f, th, method="Powell", bounds=list(zip(lo, hi)),
                           options=dict(maxiter=3000, xtol=1e-3, ftol=1e-6))
            th = np.clip(res.x, lo, hi)
        save_theta(th, f"{out}.json")
        pickle.dump(gs, open(f"{out}_gs.pkl", "wb"))
    return th, gs


if __name__ == "__main__":
    disallow = sys.argv[1] == "A"
    gate_name = sys.argv[2]            # all | no_WX | none
    tag = sys.argv[3]
    cfg = FL.Cfg(disallow=disallow, gates="all" if gate_name == "geo" else "no_WX")
    th0 = load_theta("theta_A.json" if disallow else "theta_B.json")
    gs0 = None
    if os.path.exists(f"theta_{tag}.json"):
        th0 = load_theta(f"theta_{tag}.json"); gs0 = pickle.load(open(f"theta_{tag}_gs.pkl", "rb"))
        print("resumed", len(gs0), flush=True)
    t = time.time()
    th, gs = run(cfg, gate_name, th0, out=f"theta_{tag}", log=lambda s: print(s, flush=True), gs=gs0, rounds=30)
    print("done", time.time() - t)
