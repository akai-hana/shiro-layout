"""
search.py — Simulated annealing over letter placements.

The 26 letters are assigned to the 26 non-punctuation slots; punctuation
occupies a fixed dead-key set (configurable). Objective: trigram-DP cost
(context-aware alt-fingering per trigram-context rules Y/Z) + finger-balance
penalty.
"""
import json
import math
import random
import time
import numpy as np

from fingers import KEY_ORDER, KEYS, IDEAL_SHARE
from scorer import get_tables, Corpus, SPACE

LETTERS = "abcdefghijklmnopqrstuvwxyz"

# balance/hand penalty weights (module constants for robustness sweeps)
W_BALANCE = 1.5
W_HAND = 0.5


class Evaluator:
    def __init__(self, corpus, dead_keys):
        """dead_keys: dict char->key name for punctuation occupying slots."""
        self.corpus = corpus
        self.slots = [k for k in KEY_ORDER if k not in dead_keys.values()]
        assert len(self.slots) == 26, len(self.slots)
        self.dead = dead_keys
        BIG, TRI, _, _ = get_tables()
        self.TRI = TRI
        self.BIG = BIG
        # unique trigram types as arrays
        tri_ids = []
        tri_cnt = []
        for tg, cnt in corpus.T.items():
            if len(tg) == 3 and all(c in LETTERS for c in tg):
                tri_ids.append(tg)
                tri_cnt.append(cnt)
        self.tri_types = tri_ids
        self.tri_counts = np.array(tri_cnt, dtype=np.float64)
        self.n_tri = self.tri_counts.sum()
        # pre-encode letters as indices 0..25
        self.tri_enc = np.array([[ord(a) - 97, ord(b) - 97, ord(c) - 97]
                                 for a, b, c in tri_ids], dtype=np.int32)
        # letter frequencies for balance
        self.lfreq = np.array([corpus.L.get(ch, 0.0) for ch in LETTERS])
        self.lfreq = self.lfreq / self.lfreq.sum()
        self._tri_flat_cache = {}

    def _key_ids(self, perm):
        """perm: tuple of 26 slot indices (positions in self.slots) for letters a-z.
        Returns np array letter->key_id."""
        key_ids = np.empty(26, dtype=np.int32)
        for li, slot_idx in enumerate(perm):
            key = self.slots[slot_idx]
            key_ids[li] = KEY_ORDER.index(key)
        return key_ids

    def score(self, perm):
        kid = self._key_ids(perm)
        a = kid[self.tri_enc[:, 0]]
        b = kid[self.tri_enc[:, 1]]
        c = kid[self.tri_enc[:, 2]]
        flat = a * N2 + b * N1 + c
        cost = float((self.TRI.ravel()[flat] * self.tri_counts).sum())
        cost /= self.n_tri
        # balance penalty using default fingers of chosen keys
        usage = {"L": np.zeros(4), "R": np.zeros(4)}  # P R M I
        fidx = {"P": 0, "R": 1, "M": 2, "I": 3}
        for li in range(26):
            k = KEY_ORDER[kid[li]]
            f = KEYS[k].alts[0][0]
            h = KEYS[k].hand
            usage[h][fidx[f]] += self.lfreq[li]
        pen = 0.0
        for h in "LR":
            hs = usage[h].sum()
            if hs <= 0:
                continue
            for fi in range(4):
                share = usage[h][fi] / hs
                ideal = IDEAL_SHARE[["P", "R", "M", "I"][fi]]
                pen += ((share - ideal) / ideal) ** 2
        pen *= W_BALANCE
        # hand balance (want ~50/50)
        hl, hr = usage["L"].sum(), usage["R"].sum()
        pen += ((hl - hr) / max(hl + hr, 1e-9)) ** 2 * W_HAND
        return cost + pen

    def anneal(self, seed_perm=None, iters=60000, t0=0.35, t1=0.002,
               seed=0, log_every=0):
        rng = random.Random(seed)
        perm = list(seed_perm) if seed_perm else list(range(26))
        if seed_perm is None:
            rng.shuffle(perm)
        best = perm[:]
        best_s = self.score(perm)
        cur_s = best_s
        for it in range(iters):
            T = t0 * (t1 / t0) ** (it / iters)
            i, j = rng.sample(range(26), 2)
            perm[i], perm[j] = perm[j], perm[i]
            s = self.score(perm)
            if s < cur_s or rng.random() < math.exp((cur_s - s) / T):
                cur_s = s
                if s < best_s:
                    best_s = s
                    best = perm[:]
            else:
                perm[i], perm[j] = perm[j], perm[i]
            if log_every and it % log_every == 0:
                print(f"  it {it:6d} T={T:.4f} cur={cur_s:.4f} best={best_s:.4f}")
        return best, best_s

    def layout_str(self, perm):
        out = list(self.dead.keys())  # not used; build properly below
        arr = [None] * 30
        for ch, kname in self.dead.items():
            arr[KEY_ORDER.index(kname)] = ch
        for li, slot_idx in enumerate(perm):
            arr[KEY_ORDER.index(self.slots[slot_idx])] = LETTERS[li]
        return "".join(arr)


N1 = 31
N2 = 31 * 31


def random_search(evaluator, n_restarts=12, iters=45000, base_seed=1, verbose=True):
    results = []
    for r in range(n_restarts):
        t = time.time()
        best, s = evaluator.anneal(iters=iters, seed=base_seed * 100 + r)
        layout = evaluator.layout_str(best)
        dt = time.time() - t
        results.append((s, layout))
        if verbose:
            print(f"restart {r}: score={s:.4f} ({dt:.1f}s)  {layout}")
    results.sort(key=lambda x: x[0])
    return results
