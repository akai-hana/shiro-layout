"""Fit pattern parameters to all active rulings (gates as explicit +12 context table)."""
import sys, os, json, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import numpy as np
from scipy.optimize import minimize, differential_evolution
from utils import comfort as cg, fitlib as FL
try:
    from . import fit2
except ImportError:
    import fit2

REG = 0.02
disallow = sys.argv[1] == "A"; gates = sys.argv[2]; tag = sys.argv[3]
cfg = FL.Cfg(disallow, gates)
# the user's own declared constants (comfort.py): travel 0.2*(0.7 down + 0.4 up + 1.0 lateral),
# roll rewards / leap penalties.  Not fitted.
FREEZE = {"e_dn": 0.14, "e_up_I": 0.08, "e_up_P": 1.0, "e_lat": 0.20,
          "r_MI": -0.275, "r_RM_in": -0.35, "r_RM_out": -0.20, "r_PR_out": -0.14, "r_PR_in": 0.06,
          "leap2": 0.15, "leap3": 0.35, "leap_P": 0.15}
lo, hi = cg.LO.copy(), cg.HI.copy()
for n, v in FREEZE.items():
    i = cg.PNAMES.index(n); lo[i] = hi[i] = v
for n in ("e_lat2", "e_unl", "e_dn2", "e_ov", "e_nd", "e_nd_rest", "e_ndgap", "shift_disc", "nd_waive", "shift_thr"):
    hi[cg.PNAMES.index(n)] = min(hi[cg.PNAMES.index(n)], 1.0)
f = lambda x: FL.total_loss(np.clip(x, lo, hi), cfg, reg=REG)
th0 = fit2.load_theta(f"theta3_{tag}.json") if os.path.exists(f"theta3_{tag}.json") else fit2.load_theta("theta_A.json" if disallow else "theta_B.json")
th0 = np.clip(th0, lo, hi)
best, bl = th0, f(th0)
print("start", bl, flush=True)
rng = np.random.default_rng(0)
t0 = time.time()
for it in range(6):
    start = best if it == 0 else np.clip(best + rng.normal(0, 0.05, len(best)) * (hi - lo), lo, hi)
    r = minimize(f, start, method="Powell", bounds=list(zip(lo, hi)), options=dict(maxiter=4000, xtol=1e-3, ftol=1e-7))
    if r.fun < bl:
        best, bl = np.clip(r.x, lo, hi), r.fun
    l, det = FL.total_loss(best, cfg, reg=REG, return_detail=True)
    print(f"it {it} loss {bl:.4f} fails {sum(not d[1] for d in det)}/{len(det)}  t={time.time()-t0:.0f}s", flush=True)
    fit2.save_theta(best, f"theta3_{tag}.json")
    if all(d[1] for d in det) and it >= 1:
        break
for r_, ok, lo_, txt in det:
    if not ok: print("FAIL", r_.id, txt)
