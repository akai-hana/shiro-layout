#!/usr/bin/env python3
"""D2: dead-set search — which 4 punct chars occupy the grid, and where.

Config = (4 chars on grid) x (which slot the 4th char takes):
  core chars {',', '.', '/'} always on their natural keys;
  4th char X in {';', "'", '-'} at slot S in {'Q', 'P'}  -> 6 configs.
Letters + punct positions annealed jointly under the punct-priced
objective (corpus/punct2). Baseline per config = cfgE-family seed.
Every winner also scored on the pure-English axis (Norvig letters-only).
"""
import json
import random
import sys
import time

import numpy as np

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer import Corpus, get_tables
from search import Evaluator, LETTERS
from fingers import KEY_ORDER, KEYS, IDEAL_SHARE
from compare import dead_of
from robustness import quench
from search_punct import quench30

P2 = "/home/akai/git/layout-opencode-investigation/corpus/punct2/"
EN = "/home/akai/git/layout-opencode-investigation/corpus/"
CFGE = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
CFGG = "faiojwlrn;hgpeyvtcmsbqxuzkd,./"
CORE = ",./"
CHARS_X = [";", "'", "-"]
SLOTS_S = ["Q", "P"]
N1_ = 31


class Feval:
    """Full 30-char evaluator parameterized by its punct char set."""

    def __init__(self, corpus, sigma_punct):
        self.punct = "".join(sigma_punct)                 # 4 chars
        self.sigma = "abcdefghijklmnopqrstuvwxyz" + self.punct
        self.cid = {c: i for i, c in enumerate(self.sigma)}
        tri_ids, tri_cnt = [], []
        for tg, cnt in corpus.T.items():
            if len(tg) == 3 and all(c in self.cid for c in tg):
                tri_ids.append(tg)
                tri_cnt.append(cnt)
        self.tri_counts = np.array(tri_cnt, dtype=np.float64)
        self.n_tri = self.tri_counts.sum()
        self.tri_enc = np.array([[self.cid[a], self.cid[b], self.cid[c]]
                                 for a, b, c in tri_ids], dtype=np.int32)
        lfreq = np.array([corpus.L.get(ch, 0.0) for ch in self.sigma])
        self.lfreq = lfreq / lfreq.sum()
        BIG, TRI, _, _ = get_tables()
        self.TRI = TRI

    def score(self, state):
        kid = np.asarray(state, dtype=np.int32)
        a = kid[self.tri_enc[:, 0]]
        b = kid[self.tri_enc[:, 1]]
        c = kid[self.tri_enc[:, 2]]
        flat = a * N1_ * N1_ + b * N1_ + c
        cost = float((self.TRI.ravel()[flat] * self.tri_counts).sum())
        cost /= self.n_tri
        usage = {"L": np.zeros(4), "R": np.zeros(4)}
        fidx = {"P": 0, "R": 1, "M": 2, "I": 3}
        for ci in range(30):
            k = KEY_ORDER[kid[ci]]
            f = KEYS[k].alts[0][0]
            usage[KEYS[k].hand][fidx[f]] += self.lfreq[ci]
        pen = 0.0
        for h in "LR":
            hs = usage[h].sum()
            if hs <= 0:
                continue
            for fi in range(4):
                share = usage[h][fi] / hs
                ideal = IDEAL_SHARE[["P", "R", "M", "I"][fi]]
                pen += ((share - ideal) / ideal) ** 2
        pen *= 1.5
        hl, hr = usage["L"].sum(), usage["R"].sum()
        pen += ((hl - hr) / max(hl + hr, 1e-9)) ** 2 * 0.5
        return cost + pen

    def state_of(self, layout30):
        pos = {ch: i for i, ch in enumerate(layout30)}
        assert set(layout30) == set(self.sigma), (
            sorted(set(self.sigma) - set(layout30)),
            sorted(set(layout30) - set(self.sigma)))
        return [pos[c] for c in self.sigma]

    def layout_str(self, state):
        arr = [""] * 30
        for ci, pos in enumerate(state):
            arr[pos] = self.sigma[ci]
        return "".join(arr)

    def anneal(self, state0, iters=45000, t0=0.35, t1=0.002, seed=0):
        rng = random.Random(seed)
        state = state0[:]
        best, best_s = state[:], self.score(state)
        cur_s = best_s
        import math
        for it in range(iters):
            T = t0 * (t1 / t0) ** (it / iters)
            i, j = rng.sample(range(30), 2)
            state[i], state[j] = state[j], state[i]
            s = self.score(state)
            if s < cur_s or rng.random() < math.exp((cur_s - s) / T):
                cur_s = s
                if s < best_s:
                    best, best_s = state[:], s
            else:
                state[i], state[j] = state[j], state[i]
        return best, best_s


def letters_axis(layout30, corpus_en):
    ev = Evaluator(corpus_en, dead_of(layout30))
    perm = []
    slot_pos = {k: i for i, k in enumerate(ev.slots)}
    for ch in LETTERS:
        perm.append(slot_pos[KEY_ORDER[layout30.index(ch)]])
    return ev.score(tuple(perm))


def make_seed(X, S):
    """Build a valid30-char seed: cfgE letters, natural core punct, X@S."""
    lay = list(CFGE)
    idx_s = KEY_ORDER.index(S)
    if S == "Q":
        # cfgE has ';'@Q -> replace with X; ';' leaves alphabet; all letters intact
        lay[idx_s] = X
    elif S == "P":
        # cfgG has ';'@P; cfgE has 'l'@I... P key: cfgE puts 'n'@I? find cfgE char at P
        # cfgE P key (top right) holds 'l'? KEY_ORDER[9]='O'... compute:
        old = lay[idx_s]                     # letter currently at S in cfgE
        # place X at S; move old letter to where ';' sat in cfgE (Q), ';' leaves
        idx_q = KEY_ORDER.index("Q")
        lay[idx_q] = old
        lay[idx_s] = X
    else:
        raise ValueError(S)
    return "".join(lay)


def main():
    t0 = time.time()
    corpus_p2 = Corpus.load(P2)
    corpus_en = Corpus.load(EN)
    lines = ["D2 — dead-set search (punct2 corpus: a-z + . , / ; ' -)",
             "core punct , . / fixed on natural keys; 4th char X x slot S", ""]

    results = []
    for X in CHARS_X:
        for S in SLOTS_S:
            chars = CORE + X
            fev = Feval(corpus_p2, chars)
            seed = make_seed(X, S)
            st0 = fev.state_of(seed)
            s_seed = fev.score(st0)
            best, bs = st0[:], s_seed
            for si in range(2):
                st, ss = fev.anneal(st0, iters=45000, seed=hash((X, S, si)) % 99999)
                st, ss = quench30(fev, st, ss, random.Random(si), deep2=True)
                if ss < bs:
                    best, bs = st[:], ss
            lay = fev.layout_str(best)
            s_letters = letters_axis(lay, corpus_en)
            tag = f"X={X!r} S={S}"
            results.append((bs, s_letters, tag, lay, s_seed))
            print(f"{tag}: punct {s_seed:.4f} -> {bs:.4f}  "
                  f"letters {s_letters:.4f}  {lay}", flush=True)
            lines.append(
                f"{tag:14s} seed-punct {s_seed:.4f}  best-punct {bs:.4f}  "
                f"letters {s_letters:.4f}  {lay}")

    # baselines on punct2 (cfgE + cfgG as their own alphabets)
    for name, lay, chars in (("cfgE", CFGE, CORE + ";"), ("cfgG", CFGG, CORE + ";")):
        fev = Feval(corpus_p2, chars)
        s = fev.score(fev.state_of(lay))
        s_letters = letters_axis(lay, corpus_en)
        lines.append(f"baseline {name:4s} punct {s:.4f}  letters {s_letters:.4f}  {lay}")
        print(f"baseline {name}: punct {s:.4f} letters {s_letters:.4f}", flush=True)

    lines.append("")
    lines.append(f"({time.time()-t0:.0f}s total)")
    txt = "\n".join(lines) + "\n"
    open("/home/akai/git/layout-opencode-investigation/results/d2_deadset.txt",
         "w").write(txt)
    print(txt)


if __name__ == "__main__":
    main()
