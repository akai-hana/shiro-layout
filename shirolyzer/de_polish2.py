import sys, os, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np
from scipy.optimize import differential_evolution
import comfort_geo as cg, fitlib as FL, fit2

FREEZE = {"e_dn": 0.14, "e_up_I": 0.08, "e_up_P": 1.0, "e_lat": 0.20, "r_MI": -0.275, "r_RM_in": -0.35, "r_RM_out": -0.20,
          "r_PR_out": -0.14, "r_PR_in": 0.06, "leap2": 0.15, "leap3": 0.35, "leap_P": 0.15}
CFG = FL.Cfg(True, "all")
lo, hi = cg.LO.copy(), cg.HI.copy()
for n, v in FREEZE.items():
    i = cg.PNAMES.index(n); lo[i] = hi[i] = v
for n in ("e_lat2", "e_unl", "e_dn2", "e_ov", "e_nd", "e_nd_rest", "e_ndgap", "shift_disc", "nd_waive", "shift_thr"):
    hi[cg.PNAMES.index(n)] = min(hi[cg.PNAMES.index(n)], 1.0)
tag = sys.argv[1] if len(sys.argv) > 1 else "Aall"
TH0 = np.clip(fit2.load_theta(f"theta3_{tag}.json"), lo, hi)
FREE = [i for i in range(len(lo)) if hi[i] > lo[i]]


def f(x):
    t = TH0.copy(); t[FREE] = x
    return FL.total_loss(t, CFG, reg=0.02)


def main():
    maxiter = int(sys.argv[2]) if len(sys.argv) > 2 else 60
    best = f(TH0[FREE]); print("start", best, flush=True)
    rng = np.random.default_rng(int(os.environ.get("SEED", "1")))
    span = (hi - lo)[FREE]
    pop = np.clip(TH0[FREE] + rng.normal(0, 0.04, (len(FREE) * 6, len(FREE))) * span, lo[FREE], hi[FREE])
    pop[0] = TH0[FREE]
    t0 = time.time()
    res = differential_evolution(f, list(zip(lo[FREE], hi[FREE])), init=pop, maxiter=maxiter, tol=1e-9,
                                 mutation=(0.3, 0.8), recombination=0.8, seed=int(os.environ.get("SEED", "1")), polish=False, workers=2,
                                 updating="deferred")
    t = TH0.copy(); t[FREE] = res.x
    _, det = FL.total_loss(t, CFG, reg=0.02, return_detail=True)
    print("DE", res.fun, "fails", sum(not d[1] for d in det), "t", round(time.time() - t0), flush=True)
    if res.fun < best:
        fit2.save_theta(t, f"theta3_{tag}.json"); print("saved", flush=True)
    for r_, ok, l_, txt in det:
        if not ok:
            print("FAIL", r_.id, txt)


if __name__ == "__main__":
    main()
