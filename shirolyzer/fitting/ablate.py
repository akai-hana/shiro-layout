import sys

import numpy as np

from ..utils import fitlib as FL, comfort as cg, gates as GT
from . import fit2
from .common import input_theta
from collections import Counter
tag, dis, gname = sys.argv[1], sys.argv[2]=="A", sys.argv[3]
th = fit2.load_theta(input_theta(f"theta3_{tag}.json")); cfg = FL.Cfg(dis, gname)
l, det = FL.total_loss(th, cfg, return_detail=True)
print("WITH explicit gate table :", sum(d[1] for d in det), "/", len(det))
GT.set_explicit = lambda m: None; GT.EXPLICIT_MODE = "none"
l2, det2 = FL.total_loss(th, cfg, return_detail=True)
print("geometry alone (no gates) :", sum(d[1] for d in det2), "/", len(det2))
for r, ok, lo, txt in det2:
    if not ok: print("  FAIL", r.id, txt)
m = cg.Model(theta=th, disallow_full_MR=dis); t = m.tables()
gates = GT.gate_subset("all" if gname == "all" else "no_WX")
v = GT.scan(m, t, gates)
c = Counter(); ctot = Counter()
for keys, forb, *_ in v:
    for p, f in forb: c[(keys[p], f)] += 1
print("sequences where geometry alone breaks a gate: bigrams", sum(len(x[0])==2 for x in v), "of 900; trigrams", sum(len(x[0])==3 for x in v), "of 27000")
print("  by gated usage:", c.most_common())
