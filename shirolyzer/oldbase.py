"""Score the user's SHIPPED round-8 model (analysis/scorer2.py, with its alt-context gates)
against the same ruling list.  Read-only on the project."""
import os, sys, itertools
if len(sys.argv) > 1:
    os.environ["SHIRO_ANALYSIS_DIR"] = sys.argv[1]
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oldmodel as OM
from oldmodel import eff, opts, _bigram_cost, _trigram_cost, _alt_context_penalty
import rulings as RL

def pair(it):
    a, fa, b, fb = it
    return (eff(a, fa) + eff(b, fb) + _alt_context_penalty(a, fa, (b,)) + _alt_context_penalty(b, fb, (a,))
            + _bigram_cost(a, fa, b, fb))

def pair_only(it):
    a, fa, b, fb = it
    return _bigram_cost(a, fa, b, fb)

def rows(keys):
    n = len(keys)
    return (OM.best_bigram(*keys, topn=999) if n == 2 else OM.best_trigram(*keys, topn=999))

def check(r, cfg_all_gates=True, disallow=True):
    k = r.kind
    if k == "top1":
        typ, keys, fing = r.data
        rs = rows(keys); return rs[0][1] == fing, f"got {rs[0][1]} ({rs[0][0]:.2f})"
    if k == "cons":
        d = r.data; rs = rows(d["keys"]); n = len(d["keys"])
        def legal(f):
            return all(f[p] in fs for p, fs in d["req"]) and all(f[p] != x for p, x in d["forbid"])
        A = [c for c, f in rs if legal(f)]; B = [c for c, f in rs if not legal(f)]
        if not A: return False, "unreachable"
        mg = d["margin"]; return (min(A) - min(B) <= -mg + 1e-9), f"legal {min(A):.2f} vs other {min(B):.2f}"
    if k == "order":
        c = [pair(i) for i in r.data]; return all(c[i] < c[i+1] for i in range(len(c)-1)), " < ".join(f"{x:.2f}" for x in c)
    if k == "tie":
        c = [pair(i) for i in r.data]; return abs(c[0]-c[1]) <= 0.08, f"{c[0]:.2f} vs {c[1]:.2f}"
    if k == "infeas":
        c = pair(r.data); return c >= 8.0, f"{c:.2f}"
    if k == "feasible":
        c = pair(r.data); return c < 7.0, f"{c:.2f}"
    if k == "band":
        item, lo, hi = r.data; c = pair_only(item); return lo <= c <= hi, f"{c:.2f}"
    if k == "tiers":
        tc = [[pair(i) for i in t] for t in r.data]
        return all(min(tc[i+1]) > max(tc[i]) for i in range(len(tc)-1)), "ladder"
    if k == "key1":
        kk, f = r.data; e = {x: eff(kk, x) for x in opts(kk)}; return min(e, key=e.get) == f, str(min(e, key=e.get))
    return None, "?"

if __name__ == "__main__":
    import json
    out = {}
    for r in RL.RULINGS:
        if r.id in ("inf_ORcM",):
            pass
        ok, txt = check(r)
        out[r.id] = (ok, txt)
        if not ok: print("OLD-FAIL", r.id, txt)
    print(sum(v[0] for v in out.values()), "/", len(out))
    json.dump(out, open("oldbase.json", "w"))
