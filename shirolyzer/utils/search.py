"""Generic local search primitives for shirolyzer layouts.

The search model is corpus- and layout-agnostic: callers provide an evaluator
and a starting permutation.  Dead keys are fixed; the remaining keys form the
26-letter permutation space.
"""
import math
import random

import numpy as np

from . import comfort as cg
from . import scorer as SG
from .ansi import KEYS, KI, H
from .fingers import DEFAULT_FINGER

LETTERS = "abcdefghijklmnopqrstuvwxyz"
FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}
FORDER = {"P": 0, "R": 1, "M": 2, "I": 3}
FLAT_SHARE = {"P": 0.18, "R": 0.25, "M": 0.28, "I": 0.29}
W_BALANCE = 0.4
W_HAND = 0.5


def dead_of(layout):
    dead = {}
    extra = 0
    for i, ch in enumerate(layout):
        if ch not in LETTERS:
            key = ch
            if ch in dead:
                extra += 1
                key = f"\x01{extra}"
            dead[key] = KEYS[i]
    return dead


def slots_of(evaluator, layout):
    letters = [ch for ch in layout if ch in LETTERS]
    if len(letters) != 26 or len(set(letters)) != 26:
        raise SystemExit(f"layout {layout!r} does not contain 26 distinct letters")
    perm = [None] * 26
    for slot_idx, ch in enumerate(letters):
        perm[ord(ch) - 97] = slot_idx
    return tuple(perm)


class Evaluator:
    def __init__(self, corpus, dead_keys, tables=None):
        self.corpus = corpus
        self.dead = dict(dead_keys)
        self.slots = [k for k in KEYS if k not in self.dead.values()]
        if len(self.slots) != 26:
            raise SystemExit(f"need 26 letter slots, got {len(self.slots)}")
        big, tri, _, _ = tables or SG.get_tables()
        self.BIG, self.TRI = big, tri
        self.tri_ids, self.tri_counts = [], []
        for tg, cnt in corpus.T.items():
            if len(tg) == 3 and all(c in LETTERS for c in tg):
                self.tri_ids.append([ord(c) - 97 for c in tg])
                self.tri_counts.append(cnt)
        self.tri_ids = np.array(self.tri_ids, dtype=np.int32)
        self.tri_counts = np.array(self.tri_counts, dtype=np.float64)
        self.n_tri = self.tri_counts.sum() or 1.0
        self.lfreq = np.array([corpus.L.get(ch, 0.0) for ch in LETTERS], dtype=np.float64)
        self.lfreq /= self.lfreq.sum() or 1.0
        self._slot_key = np.array([KI[k] for k in self.slots], dtype=np.int32)
        self._default_finger = np.array(
            [FORDER[DEFAULT_FINGER[k]] for k in self.slots], dtype=np.int32)
        self._hand = np.array([int(H[KI[k]]) for k in self.slots], dtype=np.int32)
        tri = self.TRI.ravel()
        a = self.tri_ids[:, 0]
        b = self.tri_ids[:, 1]
        c = self.tri_ids[:, 2]
        self._pref_a = self.tri_counts * a
        self._pref_b = self.tri_counts * b
        self._pref_c = self.tri_counts * c
        self._pref_const = 0

    def _key_ids(self, perm):
        return self._slot_key[np.asarray(perm, dtype=np.int32)]

    def score(self, perm):
        kid = self._key_ids(perm)
        flat = (kid[self.tri_ids[:, 0]] * 961
                + kid[self.tri_ids[:, 1]] * 31
                + kid[self.tri_ids[:, 2]])
        cost = float((self.TRI.ravel()[flat] * self.tri_counts).sum() / self.n_tri)
        usage = np.zeros((2, 4))
        for li in range(26):
            usage[self._hand[li], self._default_finger[li]] += self.lfreq[li]
        pen = 0.0
        for h in range(2):
            total = usage[h].sum()
            if total <= 0:
                continue
            for fi, f in enumerate("PRMI"):
                share = usage[h, fi] / total
                pen += ((share - FLAT_SHARE[f]) / FLAT_SHARE[f]) ** 2
        pen *= W_BALANCE
        hl, hr = usage[0].sum(), usage[1].sum()
        pen += ((hl - hr) / max(hl + hr, 1e-9)) ** 2 * W_HAND
        return cost + pen

    def anneal(self, seed_perm=None, iters=60000, t0=0.35, t1=0.002, seed=0):
        rng = random.Random(seed)
        perm = list(seed_perm) if seed_perm else list(range(26))
        if seed_perm is None:
            rng.shuffle(perm)
        best, cur = perm[:], self.score(perm)
        best_s = cur
        for it in range(iters):
            t = t0 * (t1 / t0) ** (it / iters)
            i, j = rng.sample(range(26), 2)
            perm[i], perm[j] = perm[j], perm[i]
            s = self.score(perm)
            if s < cur or rng.random() < math.exp((cur - s) / t):
                cur = s
                if s < best_s:
                    best_s, best = s, perm[:]
            else:
                perm[i], perm[j] = perm[j], perm[i]
        return best, best_s

    def layout_str(self, perm):
        arr = [None] * 30
        for ch, kname in self.dead.items():
            arr[KI[kname]] = ch
        for li, slot_idx in enumerate(perm):
            arr[KI[self.slots[slot_idx]]] = LETTERS[li]
        return "".join(arr)


def quench(ev, perm, cur_s, rng):
    def improve(moves, apply, undo):
        nonlocal perm, cur_s
        changed = True
        while changed:
            changed = False
            order = list(moves)
            rng.shuffle(order)
            for move in order:
                state = apply(perm, move)
                s = ev.score(perm)
                if s < cur_s - 1e-12:
                    cur_s = s
                    changed = True
                else:
                    undo(perm, move, state)
        return perm, cur_s

    def swap(p, m):
        i, j = m
        p[i], p[j] = p[j], p[i]

    def swap_undo(p, m, _):
        swap(p, m)

    pairs = [(i, j) for i in range(26) for j in range(i + 1, 26)]
    perm, cur_s = improve(pairs, swap, swap_undo)
    two = []
    for x in range(len(pairs)):
        i1, j1 = pairs[x]
        for y in range(x + 1, len(pairs)):
            i2, j2 = pairs[y]
            if len({i1, j1, i2, j2}) == 4:
                two.append((i1, j1, i2, j2))
    rng.shuffle(two)

    def swap2(p, m):
        i1, j1, i2, j2 = m
        p[i1], p[j1] = p[j1], p[i1]
        p[i2], p[j2] = p[j2], p[i2]

    def swap2_undo(p, m, _):
        swap2(p, m)
    perm, cur_s = improve(two, swap2, swap2_undo)
    three = [(i, j, k) for i in range(26) for j in range(i + 1, 26) for k in range(j + 1, 26)]

    def cycle3(p, m):
        i, j, k = m
        state = (p[i], p[j], p[k])
        p[i], p[j], p[k] = state[1], state[2], state[0]
        return state

    def cycle3_undo(p, m, state):
        i, j, k = m
        p[i], p[j], p[k] = state
    return improve(three, cycle3, cycle3_undo)


def perturb(perm, rng, k):
    cand = perm[:]
    idx = list(range(26))
    rng.shuffle(idx)
    moved = idx[:2 * k]
    for a in range(0, len(moved) - 1, 2):
        i, j = moved[a], moved[a + 1]
        cand[i], cand[j] = cand[j], cand[i]
    return cand
