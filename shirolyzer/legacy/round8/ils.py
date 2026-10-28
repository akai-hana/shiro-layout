"""Local-search primitives used by the Shai annealing pipeline.

`quench` performs first-improvement 1-swap, 2-swap, and 3-cycle descent.
`perturb` applies a random k-swap for iterated local search.
"""

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
