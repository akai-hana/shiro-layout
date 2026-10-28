#!/usr/bin/env python3
"""Round-7 anneal campaign under the round-6b scorer2 model.

Multi-start anneal (random + near-best) followed by 1/2/3-swap quench, then
ILS perturbations around the best. Re-establishes the model floor after the
X-W stagger favor, symmetric M/R full-span scissor, downward-SFB offset, and
R/P reverse full-span impossible rulings.
"""
import json
import random
import sys
import time

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer2 import Corpus
from search2 import Evaluator
from compare import dead_of, slots_of

CFG_E = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
ANNEAL_BEST = ";stckzdaiwnlprhyeguobfvmjqx,./"
NEW_BEST = ";stpbqdaiynhcrmkeguowvflzxj,./"
PREV_BEST = ";stckqdaiwnlprhyeguomfvbzxj,./"
OUT = "/home/akai/git/layout-opencode-investigation/results/anneal_r7.txt"
OUT_JSON = "/home/akai/git/layout-opencode-investigation/results/anneal_r7.json"
N = 26


def quench(ev, perm, cur_s, rng):
    def first_improve(perm, cur_s, moves, apply, undo):
        improved = True
        while improved:
            improved = False
            order = list(moves)
            rng.shuffle(order)
            for m in order:
                st = apply(perm, m)
                s = ev.score(tuple(perm))
                if s < cur_s - 1e-12:
                    cur_s = s
                    improved = True
                else:
                    undo(perm, m, st)
        return perm, cur_s

    def ap_swap(p, m):
        i, j = m
        p[i], p[j] = p[j], p[i]
        return None

    def un_swap(p, m, st):
        i, j = m
        p[i], p[j] = p[j], p[i]

    perm, cur_s = first_improve(
        perm, cur_s, [(i, j) for i in range(N) for j in range(i + 1, N)],
        ap_swap, un_swap)
    prs = [(i, j) for i in range(N) for j in range(i + 1, N)]
    two = []
    for a in range(len(prs)):
        i1, j1 = prs[a]
        for b in range(a + 1, len(prs)):
            i2, j2 = prs[b]
            if len({i1, j1, i2, j2}) == 4:
                two.append((i1, j1, i2, j2))
    rng.shuffle(two)

    def ap_two(p, m):
        i1, j1, i2, j2 = m
        p[i1], p[j1] = p[j1], p[i1]
        p[i2], p[j2] = p[j2], p[i2]
        return None

    def un_two(p, m, st):
        ap_two(p, m)

    perm, cur_s = first_improve(perm, cur_s, two, ap_two, un_two)
    three = [(i, j, k) for i in range(N) for j in range(i + 1, N)
             for k in range(j + 1, N)]

    def ap_cyc3(p, m):
        i, j, k = m
        st = (p[i], p[j], p[k])
        p[i], p[j], p[k] = st[1], st[2], st[0]
        return st

    def un_cyc3(p, m, st):
        i, j, k = m
        p[i], p[j], p[k] = st

    perm, cur_s = first_improve(perm, cur_s, three, ap_cyc3, un_cyc3)
    return perm, cur_s


def perturb(perm, rng, k):
    cand = perm[:]
    idx = list(range(N))
    rng.shuffle(idx)
    moved = idx[:2 * k]
    for a in range(0, len(moved) - 1, 2):
        i, j = moved[a], moved[a + 1]
        cand[i], cand[j] = cand[j], cand[i]
    return cand


def main():
    n_runs = int(sys.argv[1]) if len(sys.argv) > 1 else 16
    iters = int(sys.argv[2]) if len(sys.argv) > 2 else 100000
    corpus = Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")
    ev = Evaluator(corpus, dead_of(CFG_E))
    rng = random.Random(715)
    t0 = time.time()

    starts = [("NEW_BEST", list(slots_of(ev, NEW_BEST))),
              ("PREV_BEST", list(slots_of(ev, PREV_BEST))),
              ("annealBEST", list(slots_of(ev, ANNEAL_BEST)))]
    for s in range(n_runs):
        perm = list(range(N))
        random.Random(2000 + s).shuffle(perm)
        starts.append((f"rand{2000+s}", perm))

    results = []
    print(f"{'start':12s} {'quench':>8s} {'anneal':>8s} {'q2':>8s}  layout",
          flush=True)
    for name, start in starts:
        ts = time.time()
        cur, cs = quench(ev, start[:], ev.score(tuple(start)), random.Random(7))
        aperm, ans = ev.anneal(seed_perm=cur, iters=iters, seed=11)
        aperm, q2 = quench(ev, aperm, ans, random.Random(17))
        lay = ev.layout_str(tuple(aperm))
        row = (name, cs, ans, q2, lay)
        results.append(row)
        print(f"{name:12s} {cs:8.4f} {ans:8.4f} {q2:8.4f}  {lay}  "
              f"({time.time()-ts:.0f}s)", flush=True)
        if q2 < ev.score(tuple(slots_of(ev, NEW_BEST))) - 1e-9:
            print("  *** BEATS NEW_BEST", flush=True)

    # ILS around the top-5 unique layouts
    uniq = {}
    for name, cs, ans, q2, lay in results:
        if lay not in uniq or q2 < uniq[lay][0]:
            uniq[lay] = (q2, name)
    top = sorted(((s, lay, name) for lay, (s, name) in uniq.items()))[:5]
    print("\nILS around top-5:", flush=True)
    for rank, (s, lay, name) in enumerate(top, 1):
        perm = list(slots_of(ev, lay))
        best, best_s = perm[:], s
        for it in range(8):
            cand = perturb(best, rng, rng.choice([4, 6, 8, 10]))
            cand, cs = quench(ev, cand, ev.score(tuple(cand)), rng)
            cand, cs = ev.anneal(seed_perm=cand, iters=iters // 2, seed=31 + it)
            cand, cs = quench(ev, cand, cs, rng)
            if cs < best_s - 1e-12:
                best, best_s = cand[:], cs
        ilay = ev.layout_str(tuple(best))
        print(f"  ILS{rank} {name:12s} {s:.4f} -> {best_s:.4f}  {ilay}",
              flush=True)
        results.append((f"ILS{rank}:{name}", s, best_s, best_s, ilay))
        if best_s < ev.score(tuple(slots_of(ev, NEW_BEST))) - 1e-9:
            print("  *** BEATS NEW_BEST", flush=True)

    results.sort(key=lambda r: r[3])
    lines = [f"round-7 anneal campaign: {len(starts)} starts x {iters} iters "
             f"+ quench, ILS top-5",
             f"NEW_BEST score (fixed) = "
             f"{ev.score(tuple(slots_of(ev, NEW_BEST))):.6f}",
             f"{'start':20s} {'quench':>8s} {'anneal':>8s} {'final':>8s}  layout"]
    for name, cs, ans, q2, lay in results:
        lines.append(f"{name:20s} {cs:8.4f} {ans:8.4f} {q2:8.4f}  {lay}")
    txt = "\n".join(lines) + "\n"
    open(OUT, "w").write(txt)
    json.dump([{"start": n, "quench": c, "anneal": a, "final": f, "layout": l}
               for n, c, a, f, l in results], open(OUT_JSON, "w"), indent=1)
    print(f"\nbest: {results[0][3]:.4f}  {results[0][4]}")
    print(f"wrote {OUT}  ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
