#!/usr/bin/env python3
"""B6: punctuation-priced search — full 30-char annealing.

Objective change vs the archived search: the alphabet is Sigma =
a-z + {',', '.', ';', '/'} (apostrophe stays off-grid on ISO-102 and is
uniformly excluded). Trigrams/bigrams containing punct are now scored, and
punct monogram mass counts toward finger/hand balance. All 30 characters are
movable — punctuation placement is decided by the same anneal+quench that
decided the letters.

Checks:
  - static punct-aware scores of cfgE/cfgG and hand-built seeds
    (home-punct: ','->RM home, '.'->RR home; index-punct: ',' '.' -> right
    index bottom, i.e. the Engram-style counter-move),
  - quench + anneal per seed, 2 fresh random restarts,
  - letter-only score (original objective) of every final layout, so the
    English-compromise is visible on the same row.
"""
import json
import math
import random
import sys
import time

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
import numpy as np
import scorer
from scorer import Corpus, get_tables
from search import Evaluator, LETTERS
from compare import dead_of, score_of

OUT_TXT = "/home/akai/git/layout-opencode-investigation/results/b6_punct.txt"
OUT_JSON = "/home/akai/git/layout-opencode-investigation/results/b6_punct.json"

PUNCT = [",", ".", ";", "/"]
SIGMA = list(LETTERS) + PUNCT           # 30 chars, char id = index
CID = {c: i for i, c in enumerate(SIGMA)}
N1 = 31

CFG_E = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
CFG_G = "faiojwlrn;hgpeyvtcmsbqxuzkd,./"


def swap_chars(lay, a, b):
    i, j = lay.index(a), lay.index(b)
    l = list(lay)
    l[i], l[j] = l[j], l[i]
    return "".join(l)


# seeds: same Sigma, different punct philosophy
SEEDS = {
    "cfgE (as-is)": CFG_E,
    "cfgG": CFG_G,
    "home-punct": swap_chars(swap_chars(CFG_E, ",", "c"), ".", "m"),
    "index-punct": swap_chars(swap_chars(CFG_E, ",", "k"), ".", "d"),
}


class FullEvaluator:
    """30-char (letters+punct) evaluator over the same 31-key cost tables."""

    def __init__(self, corpus):
        self.corpus = corpus
        tri_ids, tri_cnt = [], []
        for tg, cnt in corpus.T.items():
            if len(tg) == 3 and all(c in CID for c in tg):
                tri_ids.append(tg)
                tri_cnt.append(cnt)
        self.tri_counts = np.array(tri_cnt, dtype=np.float64)
        self.n_tri = self.tri_counts.sum()
        self.tri_enc = np.array([[CID[a], CID[b], CID[c]]
                                 for a, b, c in tri_ids], dtype=np.int32)
        lfreq = np.array([corpus.L.get(ch, 0.0) for ch in SIGMA])
        self.lfreq = lfreq / lfreq.sum()
        BIG, TRI, _, _ = get_tables()
        self.TRI = TRI

    def kid_of(self, state):
        # state: char-id -> position; build key-id array
        return np.array(state, dtype=np.int32)

    def score(self, state):
        """state: list len 30, state[ci] = key position of char ci."""
        kid = np.asarray(state, dtype=np.int32)
        a = kid[self.tri_enc[:, 0]]
        b = kid[self.tri_enc[:, 1]]
        c = kid[self.tri_enc[:, 2]]
        flat = a * N1 * N1 + b * N1 + c
        cost = float((self.TRI.ravel()[flat] * self.tri_counts).sum())
        cost /= self.n_tri
        from fingers import KEY_ORDER, KEYS, IDEAL_SHARE
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

    @staticmethod
    def state_of(layout30):
        pos = {ch: i for i, ch in enumerate(layout30)}
        return [pos[c] for c in SIGMA]

    @staticmethod
    def layout_str(state):
        arr = [""] * 30
        for ci, pos in enumerate(state):
            arr[pos] = SIGMA[ci]
        return "".join(arr)

    def anneal(self, state0=None, iters=45000, t0=0.35, t1=0.002,
               seed=0, rng=None):
        rng = rng or random.Random(seed)
        state = state0[:] if state0 else list(range(30))
        if state0 is None:
            rng.shuffle(state)
        best, best_s = state[:], self.score(state)
        cur_s = best_s
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


def quench30(ev, state, cur_s, rng, deep2=True):
    n = 30

    def first_improve(state, cur_s, pairs):
        improved = True
        while improved:
            improved = False
            order = list(pairs)
            rng.shuffle(order)
            for i, j in order:
                state[i], state[j] = state[j], state[i]
                s = ev.score(state)
                if s < cur_s - 1e-12:
                    cur_s, improved = s, True
                else:
                    state[i], state[j] = state[j], state[i]
        return state, cur_s

    state, cur_s = first_improve(
        state, cur_s, [(i, j) for i in range(n) for j in range(i + 1, n)])
    if deep2:
        prs = [(i, j) for i in range(n) for j in range(i + 1, n)]
        two = []
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
                state[i1], state[j1] = state[j1], state[i1]
                state[i2], state[j2] = state[j2], state[i2]
                s = ev.score(state)
                if s < cur_s - 1e-12:
                    cur_s, improved = s, True
                else:
                    state[i1], state[j1] = state[j1], state[i1]
                    state[i2], state[j2] = state[j2], state[i2]
    three = [(i, j, k) for i in range(n) for j in range(i + 1, n)
             for k in range(j + 1, n)]
    rng.shuffle(three)
    improved = True
    while improved:
        improved = False
        for (i, j, k) in three:
            a, b, c = state[i], state[j], state[k]
            state[i], state[j], state[k] = b, c, a
            s = ev.score(state)
            if s < cur_s - 1e-12:
                cur_s, improved = s, True
            else:
                state[i], state[j], state[k] = a, b, c
    return state, cur_s


def letter_only(layout):
    corpus = Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")
    ev = Evaluator(corpus, dead_of(layout))
    return score_of(ev, layout)


def punct_load(lay):
    """default-finger punct share: which finger types , . ; / """
    from fingers import KEY_ORDER, KEYS
    out = {}
    for ch in PUNCT:
        k = KEY_ORDER[lay.index(ch)]
        f = KEYS[k].alts[0][0]
        out[ch] = f"{KEYS[k].hand}{f}@{k}"
    return out


def main():
    pcorpus = Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/punct/")
    ev = FullEvaluator(pcorpus)
    lines = ["B6 — punct-aware 30-char search (Sigma = a-z + , . ; / ; "
             "' excluded)", ""]
    results = {}

    # static scores
    lines.append("--- static (seed states)")
    for name, lay in SEEDS.items():
        st = FullEvaluator.state_of(lay)
        ps = ev.score(st)
        results[name] = {"punct_score": ps, "layout": lay}
        print(f"{name:24s} punct {ps:.4f}  {lay}", flush=True)
        lines.append(f"{name:24s} punct {ps:.4f}  {lay}")

    # quench + anneal per seed
    finals = {}
    for name, lay in SEEDS.items():
        t0 = time.time()
        st = FullEvaluator.state_of(lay)
        s0 = ev.score(st)
        st, sq = quench30(ev, st, s0, random.Random(7), deep2=True)
        st2, sa = ev.anneal(state0=st, iters=30000, seed=11)
        if sa < sq - 1e-12:
            st, sa = st2, sa
        flay = FullEvaluator.layout_str(st)
        finals[f"{name} -> anneal"] = {"punct_score": sa, "layout": flay,
                                       "from": s0, "quench": sq,
                                       "secs": time.time() - t0}
        print(f"{name:24s} {s0:.4f} -> quench {sq:.4f} -> anneal {sa:.4f} "
              f"({time.time()-t0:.0f}s)  {flay}", flush=True)
        lines.append(f"{name:24s} {s0:.4f} -> quench {sq:.4f} -> "
                     f"anneal {sa:.4f}  {flay}")

    # fresh random restarts
    for r in range(2):
        t0 = time.time()
        st, sa = ev.anneal(state0=None, iters=45000, seed=100 + r)
        st, sq = quench30(ev, st, sa, random.Random(200 + r), deep2=False)
        flay = FullEvaluator.layout_str(st)
        finals[f"random {r}"] = {"punct_score": sq, "layout": flay,
                                 "secs": time.time() - t0}
        print(f"random {r}: anneal {sa:.4f} -> quench {sq:.4f} "
              f"({time.time()-t0:.0f}s)  {flay}", flush=True)
        lines.append(f"random {r}: anneal {sa:.4f} -> quench {sq:.4f}  {flay}")

    # best + compromise table (both axes)
    best_name, best = min(
        ((k, v) for k, v in finals.items()),
        key=lambda kv: kv[1]["punct_score"])
    lines.append("")
    lines.append("--- finals on both axes (letter-only = original objective)")
    table = dict(results)
    table.update(finals)
    rows = sorted(table.items(), key=lambda kv: kv[1]["punct_score"])
    for name, v in rows:
        lay = v["layout"]
        ls = letter_only(lay)
        v["letter_score"] = ls
        lines.append(f"{name:34s} punct {v['punct_score']:7.4f}  "
                     f"letters {ls:7.4f}  punct keys: {punct_load(lay)}")
        print(lines[-1], flush=True)
    lines.append("")
    lines.append(f"BEST: {best_name}  punct {best['punct_score']:.4f}  "
                 f"letters {best['letter_score']:.4f}  {best['layout']}")
    lines.append(f"cfgE letters-only baseline 4.3877; "
                 f"cfgE punct-aware {results['cfgE (as-is)']['punct_score']:.4f}")

    txt = "\n".join(lines) + "\n"
    with open(OUT_TXT, "w") as f:
        f.write(txt)
    json.dump({"statics": results, "finals": finals}, open(OUT_JSON, "w"),
              indent=1)
    print("wrote", OUT_TXT)


if __name__ == "__main__":
    main()
