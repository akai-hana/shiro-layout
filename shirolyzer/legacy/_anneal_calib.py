#!/usr/bin/env python3
"""Legacy calibrated anneal (now Shai primary, Spanish tie-break)."""
import json
import random
import sys
import time

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from compare import dead_of, slots_of
from objective import SECONDARY_WEIGHT, build_evaluators
from _anneal_r7 import quench, perturb

CFG_E = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
ANNEAL_BEST = ";stckzdaiwnlprhyeguobfvmjqx,./"
NEW_BEST = ";stpbqdaiynhcrmkeguowvflzxj,./"
PREV_BEST = ";stckqdaiwnlprhyeguomfvbzxj,./"
OUT = "/home/akai/git/layout-opencode-investigation/results/anneal_calib.txt"
OUT_JSON = "/home/akai/git/layout-opencode-investigation/results/anneal_calib.json"
N = 26


def main():
    n_runs = int(sys.argv[1]) if len(sys.argv) > 1 else 8
    iters = int(sys.argv[2]) if len(sys.argv) > 2 else 80000
    ev, pev, scale = build_evaluators(dead_of(CFG_E), SECONDARY_WEIGHT)
    rng = random.Random(916)
    t0 = time.time()
    print(f"secondary scale={scale:.6g} weight={SECONDARY_WEIGHT}", flush=True)

    seeds = [("NEW_BEST", NEW_BEST), ("PREV_BEST", PREV_BEST),
             ("annealBEST", ANNEAL_BEST), ("cfgE", CFG_E)]
    starts = [(name, list(slots_of(ev, lay))) for name, lay in seeds]
    for s in range(n_runs):
        perm = list(range(N))
        random.Random(4100 + s).shuffle(perm)
        starts.append((f"rand{4100+s}", perm))

    results = []
    print(f"{'start':12s} {'quench':>8s} {'anneal':>8s} {'q2':>8s} {'primary':>8s}  layout",
          flush=True)
    for name, start in starts:
        ts = time.time()
        cur, cs = quench(ev, start[:], ev.score(tuple(start)), random.Random(7))
        aperm, ans = ev.anneal(seed_perm=cur, iters=iters, seed=17)
        aperm, q2 = quench(ev, aperm, ans, random.Random(27))
        lay = ev.layout_str(tuple(aperm))
        pri = pev.score(tuple(slots_of(ev, lay)))
        results.append((name, cs, ans, q2, pri, lay))
        print(f"{name:12s} {cs:8.4f} {ans:8.4f} {q2:8.4f} {pri:8.4f}  {lay}  "
              f"({time.time()-ts:.0f}s)", flush=True)

    uniq = {}
    for name, cs, ans, q2, pri, lay in results:
        if lay not in uniq or q2 < uniq[lay][0]:
            uniq[lay] = (q2, pri, name)
    top = sorted(((s, pri, lay, name) for lay, (s, pri, name) in uniq.items()))[:4]
    print("\nILS:", flush=True)
    for rank, (s, pri, lay, name) in enumerate(top, 1):
        perm = list(slots_of(ev, lay))
        best, best_s = perm[:], s
        for it in range(6):
            cand = perturb(best, rng, rng.choice([4, 6, 8]))
            cand, cs = quench(ev, cand, ev.score(tuple(cand)), rng)
            cand, cs = ev.anneal(seed_perm=cand, iters=iters // 2, seed=51 + it)
            cand, cs = quench(ev, cand, cs, rng)
            if cs < best_s - 1e-12:
                best, best_s = cand[:], cs
        ilay = ev.layout_str(tuple(best))
        ipri = pev.score(tuple(slots_of(ev, ilay)))
        print(f"  ILS{rank} {name:12s} {s:.4f} -> {best_s:.4f} "
              f"primary={ipri:.4f}  {ilay}", flush=True)
        results.append((f"ILS{rank}:{name}", s, best_s, best_s, ipri, ilay))

    results.sort(key=lambda r: r[3])
    lines = [
        f"calibrated anneal: primary=shai, secondary=spanish w={SECONDARY_WEIGHT}",
        f"secondary scale={scale:.6g}",
        f"NEW_BEST composite={ev.score(tuple(slots_of(ev, NEW_BEST))):.6f} "
        f"primary={pev.score(tuple(slots_of(ev, NEW_BEST))):.6f}",
        f"{'start':20s} {'quench':>8s} {'anneal':>8s} {'final':>8s} {'primary':>8s}  layout",
    ]
    for name, cs, ans, q2, pri, lay in results:
        lines.append(f"{name:20s} {cs:8.4f} {ans:8.4f} {q2:8.4f} {pri:8.4f}  {lay}")
    txt = "\n".join(lines) + "\n"
    open(OUT, "w").write(txt)
    json.dump([{"start": n, "quench": c, "anneal": a, "final": f,
                "primary": p, "layout": l}
               for n, c, a, f, p, l in results], open(OUT_JSON, "w"), indent=1)
    print(f"\nbest-composite: {results[0][3]:.4f} primary={results[0][4]:.4f} "
          f"{results[0][5]}")
    print(f"wrote {OUT} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
