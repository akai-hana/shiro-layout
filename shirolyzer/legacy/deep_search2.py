#!/usr/bin/env python3
"""D1: deeper floor campaign — 4-cycle quench + multi-start ILS.

Escalation over deep_search.py (which used 1/2/3-moves + 80 ILS):
  - quench adds bidirectional 4-cycles (C(26,4) x 2 rotations)
  - ILS: perturbation = random k-swap OR double-bridge; Metropolis T=0.005
  - multiple streams: cfgE warm, 2 anneal starts, 1 random start

Goal: beat 4.3877 on the letters-only objective. Log every improvement.
"""
import random
import sys
import time

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer import Corpus
from search import Evaluator
from compare import dead_of, slots_of

WIN = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
N = 26


def layout_of(perm, ev, base=WIN):
    from fingers import KEY_ORDER
    lay = list(base)
    base_idx = {k: i for i, k in enumerate(KEY_ORDER)}
    for li, ch in enumerate("abcdefghijklmnopqrstuvwxyz"):
        pos = base_idx[ev.slots[perm[li]]]
        cur = lay.index(ch)
        if cur != pos:
            lay[pos], lay[cur] = lay[cur], lay[pos]
    return "".join(lay)


def quench4(ev, perm, cur_s, rng):
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

    # 1-swaps
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

    # disjoint 2-swaps (sampled subset each pass — full set is 52k moves)
    prs = [(i, j) for i in range(N) for j in range(i + 1, N)]
    two_all = []
    for a in range(len(prs)):
        i1, j1 = prs[a]
        for b in range(a + 1, len(prs)):
            i2, j2 = prs[b]
            if len({i1, j1, i2, j2}) == 4:
                two_all.append((i1, j1, i2, j2))
    rng.shuffle(two_all)
    two = two_all[:12000]

    def ap_two(p, m):
        i1, j1, i2, j2 = m
        p[i1], p[j1] = p[j1], p[i1]
        p[i2], p[j2] = p[j2], p[i2]
        return None

    def un_two(p, m, st):
        ap_two(p, m)

    improved = True
    while improved:
        improved = False
        for m in two:
            st = ap_two(perm, m)
            s = ev.score(tuple(perm))
            if s < cur_s - 1e-12:
                cur_s = s
                improved = True
            else:
                un_two(perm, m, st)

    # 3-cycles
    three = [(i, j, k) for i in range(N) for j in range(i + 1, N)
             for k in range(j + 1, N)]
    rng.shuffle(three)

    def ap_cyc3(p, m):
        i, j, k = m
        p[i], p[j], p[k] = p[j], p[k], p[i]
        return None

    def un_cyc3(p, m, st):
        i, j, k = m
        p[i], p[j], p[k] = st

    improved = True
    while improved:
        improved = False
        for m in three:
            st = (perm[m[0]], perm[m[1]], perm[m[2]])
            ap_cyc3(perm, m)
            s = ev.score(tuple(perm))
            if s < cur_s - 1e-12:
                cur_s = s
                improved = True
            else:
                un_cyc3(perm, m, st)

    # 4-cycles (both rotation directions)
    four = []
    for i in range(N):
        for j in range(i + 1, N):
            for k in range(j + 1, N):
                for l in range(k + 1, N):
                    four.append((i, j, k, l))
    rng.shuffle(four)

    def rot4(p, m, rev):
        i, j, k, l = m
        a, b, c, d = p[i], p[j], p[k], p[l]
        if rev:
            p[i], p[j], p[k], p[l] = d, a, b, c
        else:
            p[i], p[j], p[k], p[l] = b, c, d, a

    for rev in (False, True):
        improved = True
        while improved:
            improved = False
            for m in four:
                st = (perm[m[0]], perm[m[1]], perm[m[2]], perm[m[3]])
                rot4(perm, m, rev)
                s = ev.score(tuple(perm))
                if s < cur_s - 1e-12:
                    cur_s = s
                    improved = True
                else:
                    i, j, k, l = m
                    perm[i], perm[j], perm[k], perm[l] = st

    return perm, cur_s


def perturb(perm, rng):
    cand = perm[:]
    if rng.random() < 0.5:
        k = rng.choice([4, 5, 6, 8, 10])
        idx = list(range(N))
        rng.shuffle(idx)
        moved = idx[: 2 * k]
        for a in range(0, len(moved) - 1, 2):
            i, j = moved[a], moved[a + 1]
            cand[i], cand[j] = cand[j], cand[i]
    else:
        # double-bridge: cut 4 points, reorder segments (inclusive bounds;
        # stop=-1 would normalize to the last index and break downward slices)
        a, b, c, d = sorted(rng.sample(range(N + 1), 4))
        seg = (cand[:a] + cand[d - 1:c - 1:-1] + cand[c - 1:b - 1:-1]
               + (cand[b - 1::-1] if a == 0 else cand[b - 1:a - 1:-1])
               + cand[d:])
        cand = seg[:N]
    return cand


def main():
    stream = int(sys.argv[1]) if len(sys.argv) > 1 else -1
    n_ils = int(sys.argv[2]) if len(sys.argv) > 2 else 45
    corpus = Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")
    ev = Evaluator(corpus, dead_of(WIN))
    rng0 = random.Random(20261020 + stream)
    t_start = time.time()

    streams = []
    streams.append(("warm-cfgE", list(slots_of(ev, WIN))))
    for si in range(2):
        sp, ss = ev.anneal(iters=60000, seed=900 + si)
        streams.append((f"anneal{si}", sp))
    rnd = list(range(N))
    rng0.shuffle(rnd)
    streams.append(("random", rnd))

    names = [s[0] for s in streams]
    pick = range(len(streams)) if stream < 0 else [stream]

    results = []
    for idx in pick:
        name, start = streams[idx]
        t0 = time.time()
        cur = start[:]
        cur_s = ev.score(tuple(cur))
        cur, cur_s = quench4(ev, cur, cur_s, random.Random(hash(name) % 9999))
        print(f"[{name}] quench {cur_s:.4f}  {layout_of(cur, ev)}  "
              f"({time.time()-t0:.0f}s)", flush=True)
        best, best_s = cur[:], cur_s
        no_gain = 0
        for it in range(n_ils):
            cand = perturb(best, rng0)
            cs = ev.score(tuple(cand))
            cand, cs = quench4(ev, cand, cs, rng0)
            if cs < best_s - 1e-12:
                d = best_s - cs
                best, best_s = cand[:], cs
                print(f"[{name}] ILS {it}: {best_s:.4f} (-{d:.4f})  "
                      f"{layout_of(best, ev)}  [{time.time()-t0:.0f}s]",
                      flush=True)
                no_gain = 0
            else:
                no_gain += 1
                if no_gain >= 20:
                    break
        print(f"[{name}] stream end {best_s:.4f}  {layout_of(best, ev)}  "
              f"[{time.time()-t0:.0f}s]", flush=True)
        results.append((best_s, layout_of(best, ev), name))

    if stream < 0:
        results.sort()
        print(f"\nBEST OVERALL {results[0][0]:.6f}  {results[0][1]}")
        with open("/home/akai/git/layout-opencode-investigation/results/deep_search2_best.txt",
                  "w") as f:
            f.write(f"{results[0][0]:.6f}\n{results[0][1]}\n")
    else:
        with open(f"/home/akai/git/layout-opencode-investigation/results/deep_search2_s{stream}.txt",
                  "w") as f:
            s, lay, nm = min(results)
            f.write(f"{s:.6f}\n{lay}\n{nm}\n")


if __name__ == "__main__":
    main()
