import sys, json

import numpy as np
from scipy.optimize import minimize

from ..utils import comfort as cg, fitlib as FL, rulings as RL
from . import fit2
from .common import input_parameters, output_parameters
dis = sys.argv[1] == "A"; gates = sys.argv[2]; tag = sys.argv[3]
cfg = FL.Cfg(dis, gates); lo, hi = cg.LO, cg.HI
th_full = fit2.load_parameters(input_parameters(f"pattern_parameters_{tag}.json"))
act = [r for r in RL.RULINGS if FL.active(r, cfg)]
fams = sorted({r.family for r in act})
SKIP = lambda x: x.startswith("DEFAULT_")
res = {}
part = int(sys.argv[4]); partition = [x for x in fams if not SKIP(x)][part::2]
for fam in partition:
    keep = set(fams) - {fam}
    f = lambda x: FL.total_loss(np.clip(x, lo, hi), cfg, subset=keep, reg=0.02)
    r = minimize(f, th_full, method="Powell", bounds=list(zip(lo, hi)), options=dict(maxiter=250, maxfev=1500, xtol=1e-3, ftol=1e-6))
    th = np.clip(r.x, lo, hi)
    _, det = FL.total_loss(th, cfg, subset={fam}, return_detail=True)
    res[fam] = [(d[0].id, bool(d[1]), d[3]) for d in det]
    print(fam, [("ok" if d[1] else "MISS") for d in det], flush=True)
    json.dump(res, open(output_parameters(f"loo3_{tag}_{part}.json"), "w"))
n = sum(len(v) for v in res.values()); ok = sum(x[1] for v in res.values() for x in v)
print("LOO held-out pass", ok, "/", n)
