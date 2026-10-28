#!/usr/bin/env python3
"""Deep search from the cfgE winner: iterated local search.

Neighborhoods beyond single swaps (which provably cannot improve):
  - 2-swaps (two disjoint letter pairs exchanged)
  - 3-cycles (three letters rotated over their slots)
  - perturbation (random k-swap) + quench, Metropolis acceptance

Same objective, same dead-key map as the winner. Logs improvements.
"""
import math
import random
import sys
import time

from scorer import Corpus
from search import Evaluator, LETTERS
from compare import dead_of, slots_of
from fingers import KEY_ORDER

WIN = ";aiojvlnrwhfpeygtcmsbxquzkd,./"


def layout_of(perm, ev, base=WIN):
    lay = list(base)
    base_idx = {k: i for i, k in enumerate(KEY_ORDER)}
    for li, ch in enumerate(LETTERS):
        pos = base_idx[ev.slots[perm[li]]]
        cur = lay.index(ch)
        if cur != pos:
            lay[pos], lay[cur] = lay[cur], lay[pos]
    return "".join(lay)


def main():
    corpus = Corpus.load("../corpus/")
    ev = Evaluator(corpus, dead_of(WIN))
    base_perm = list(slots_of(ev, WIN))
    base_score = ev.score(tuple(base_perm))
    print(f"start score {base_score:.4f}", flush=True)

    rng = random.Random(12345)
    n = 26
    best = base_perm[:]
    best_s = base_score
    t_start = time.time()

    def first_improve(perm, cur_s, pairs):
        """First-improvement scan over given move generator; return new (perm,s)."""
        improved = True
        while improved:
            improved = False
            order = list(pairs)
            rng.shuffle(order)
            for move in order:
                i, j = move[0], move[1]
                perm[i], perm[j] = perm[j], perm[i]
                s = ev.score(tuple(perm))
                if s < cur_s - 1e-12:
                    cur_s = s
                    improved = True
                else:
                    perm[i], perm[j] = perm[j], perm[i]
        return perm, cur_s

    def quench(perm, cur_s):
        # 1-swap
        perm, cur_s = first_improve(perm, cur_s, [(i, j) for i in range(n) for j in range(i + 1, n)])
        # 2-swap: pairs of disjoint swaps
        two = []
        prs = [(i, j) for i in range(n) for j in range(i + 1, n)]
        for a in range(len(prs)):
            i1, j1 = prs[a]
            for b in range(a + 1, len(prs)):
                i2, j2 = prs[b]
                if len({i1, j1, i2, j2}) == 4:
                    two.append((i1, j1, i2, j2))
        rng.shuffle(two)
        improved = True
        while improved:
            improved = False
            for (i1, j1, i2, j2) in two:
                perm[i1], perm[j1] = perm[j1], perm[i1]
                perm[i2], perm[j2] = perm[j2], perm[i2]
                s = ev.score(tuple(perm))
                if s < cur_s - 1e-12:
                    cur_s = s
                    improved = True
                else:
                    perm[i1], perm[j1] = perm[j1], perm[i1]
                    perm[i2], perm[j2] = perm[j2], perm[i2]
        # 3-cycles
        three = []
        for i in range(n):
            for j in range(i + 1, n):
                for k in range(j + 1, n):
                    three.append((i, j, k))
        rng.shuffle(three)
        improved = True
        while improved:
            improved = False
            for (i, j, k) in three:
                # rotate: slot i gets letter j, slot j gets k, slot k gets i
                a, b, c = perm[i], perm[j], perm[k]
                perm[i], perm[j], perm[k] = b, c, a
                s = ev.score(tuple(perm))
                if s < cur_s - 1e-12:
                    cur_s = s
                    improved = True
                else:
                    perm[i], perm[j], perm[k] = a, b, c
        return perm, cur_s

    # full quench from the winner first
    cur = best[:]
    cur_s = best_s
    cur, cur_s = quench(cur, cur_s)
    if cur_s < best_s - 1e-12:
        best, best_s = cur[:], cur_s
        print(f"quench improved: {best_s:.4f}  {layout_of(best, ev)}", flush=True)
    else:
        print(f"quench: no improvement (still {best_s:.4f}) — confirming local opt", flush=True)

    # ILS
    no_gain = 0
    for it in range(80):
        k = rng.choice([4, 5, 6, 8])
        cand = best[:]
        idx = list(range(n))
        rng.shuffle(idx)
        moved = idx[: 2 * k]
        for a in range(0, len(moved) - 1, 2):
            i, j = moved[a], moved[a + 1]
            cand[i], cand[j] = cand[j], cand[i]
        cand_s = ev.score(tuple(cand))
        cand, cand_s = quench(cand, cand_s)
        if cand_s < best_s - 1e-12:
            d = best_s - cand_s
            best, best_s = cand[:], cand_s
            print(f"ILS {it}: {best_s:.4f} (-{d:.4f})  {layout_of(best, ev)}  "
                  f"[{time.time()-t_start:.0f}s]", flush=True)
            no_gain = 0
        else:
            no_gain += 1
        if no_gain >= 30:
            print(f"stop: 30 iters without gain at {best_s:.4f}", flush=True)
            break

    print(f"\nBEST {best_s:.4f}  {layout_of(best, ev)}", flush=True)
    with open("../results/deep_search_best.txt", "w") as f:
        f.write(f"{best_s:.6f}\n{layout_of(best, ev)}\n")


if __name__ == "__main__":
    main()
