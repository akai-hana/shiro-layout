#!/usr/bin/env python3
"""D6: space-aware objective — word boundaries enter the scorer.

SpaceEvaluator keeps trigrams containing space (space fixed on the thumb,
key id 30 = SPACE_ID). Letters still move; balance penalty stays letters-only
(thumb excluded from the 4-finger model, as designed).

Parts:
  1. score cfgE + refs + cfgE-punct + hybrid-35 under the space-aware objective
  2. anneal fresh under space-aware objective (warm cfgE + 3 random)
  3. cross-score the candidate on BOTH objectives (letters-only + space-aware)
"""
import json
import random
import sys
import time

import numpy as np

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer import Corpus, SPACE, SPACE_ID, get_tables
from search import Evaluator, LETTERS, N1, N2, W_BALANCE, W_HAND, IDEAL_SHARE
from fingers import KEY_ORDER, KEYS
from compare import dead_of, slots_of, score_of
from robustness import quench

WORD_DIR = "/home/akai/git/layout-opencode-investigation/corpus/word/"
EN_DIR = "/home/akai/git/layout-opencode-investigation/corpus/"
WIN = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
CFGG = "faiojwlrn;hgpeyvtcmsbqxuzkd,./"
CFGE_PUNCT = ".aiozvlnrbhgpeymtcwsfjxu/kd,;q"
HYBRID35 = ";aemjwcioksfgnhptubrqzylxvd,./"
REFS = {
    "cfgE (primary)": WIN,
    "cfgG": CFGG,
    "cfgE-punct": CFGE_PUNCT,
    "hybrid-35": HYBRID35,
    "Sturdy": "vmlcpxfoujsrtdy.naeizkqgwbh';,",
    "Graphite": "bldwz'foujnrtsgyhaeiqxmcvkp.-/",
    "Gallium": "bldcvjfou,nrtsgyhaeixqmwzkp';.",
    "Colemak-DH": "qwfpbjluy;arstgmneiozxcdvkh,./",
    "Canary": "wlypkzxou;crstbfneiajvdgqmh/,.",
    "Semimak": "flhvzqwuoysrntkcdeaix'bmjpg/,/",
    "QWERTY": "qwertyuiopasdfghjkl;zxcvbnm,./",
}


class SpaceEvaluator(Evaluator):
    """Trigram objective over Sigma = a-z + space (space on thumb, id 30)."""

    def __init__(self, corpus, dead_keys):
        self.corpus = corpus
        self.slots = [k for k in KEY_ORDER if k not in dead_keys.values()]
        assert len(self.slots) == 26, len(self.slots)
        self.dead = dead_keys
        BIG, TRI, _, _ = get_tables()
        self.TRI = TRI
        self.BIG = BIG
        tri_ids, tri_cnt = [], []
        for tg, cnt in corpus.T.items():
            if len(tg) == 3 and all(c in LETTERS or c == " " for c in tg):
                tri_ids.append(tg)
                tri_cnt.append(cnt)
        self.tri_types = tri_ids
        self.tri_counts = np.array(tri_cnt, dtype=np.float64)
        self.n_tri = self.tri_counts.sum()
        # 0..25 letters, 26 = space
        self.tri_enc = np.array(
            [[ord(c) - 97 if c != " " else 26 for c in tg] for tg in tri_ids],
            dtype=np.int32)
        self.lfreq = np.array([corpus.L.get(ch, 0.0) for ch in LETTERS])
        self.lfreq = self.lfreq / self.lfreq.sum()
        self._tri_flat_cache = {}

    def score(self, perm):
        kid = self._key_ids(perm)          # 26 letter -> key id
        kid_ext = np.append(kid, np.int32(SPACE_ID))   # index 26 = space
        a = kid_ext[self.tri_enc[:, 0]]
        b = kid_ext[self.tri_enc[:, 1]]
        c = kid_ext[self.tri_enc[:, 2]]
        flat = a * N2 + b * N1 + c
        cost = float((self.TRI.ravel()[flat] * self.tri_counts).sum())
        cost /= self.n_tri
        # balance + hand penalty (letters only; thumb not tracked — as designed)
        usage = {"L": np.zeros(4), "R": np.zeros(4)}
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
        hl, hr = usage["L"].sum(), usage["R"].sum()
        pen += ((hl - hr) / max(hl + hr, 1e-9)) ** 2 * W_HAND
        return cost + pen


def main():
    t_start = time.time()
    cw = Corpus.load(WORD_DIR)
    ce = Corpus.load(EN_DIR)

    def evw(layout):
        d = dead_of(layout)
        return SpaceEvaluator(cw, d)

    lines = ["D6 — space-aware objective (trigrams over a-z + space)",
             f"corpus/word: {int(sum(cw.L.values()))} chars, "
             f"space {cw.L.get(' ',0)/sum(cw.L.values())*100:.1f}%, "
             f"{len(cw.T)} trigram types "
             f"(vs letters-only {len(ce.T)})", ""]

    # --- 1. field under space-aware objective
    lines.append("-- space-aware score (lower better) --")
    rows = []
    for name, lay in REFS.items():
        s = score_of(evw(lay), lay)
        s_letters = score_of(Evaluator(ce, dead_of(lay)), lay)
        rows.append((s, s_letters, name, lay))
    rows.sort()
    lines.append(f"{'layout':16s} {'space':>7s} {'letters':>8s}")
    for s, sl, name, lay in rows:
        lines.append(f"{name:16s} {s:7.3f} {sl:8.3f}")
    lines.append("")

    # diagnostic: space-gram share of cfgE cost
    ev_cfgE = evw(WIN)
    perm = slots_of(ev_cfgE, WIN)
    kid = ev_cfgE._key_ids(perm)
    kid_ext = np.append(kid, np.int32(SPACE_ID))
    a = kid_ext[ev_cfgE.tri_enc[:, 0]]
    b = kid_ext[ev_cfgE.tri_enc[:, 1]]
    c = kid_ext[ev_cfgE.tri_enc[:, 2]]
    flat = a * N2 + b * N1 + c
    costs = ev_cfgE.TRI.ravel()[flat] * ev_cfgE.tri_counts
    enc_has = (ev_cfgE.tri_enc == 26).any(axis=1)
    total = costs.sum()
    sp = costs[enc_has].sum()
    lines.append(
        f"cfgE space-aware total {total/ev_cfgE.n_tri:.4f}; "
        f"space-adjacent grams = {100*sp/total:.1f}% of cost "
        f"({100*ev_cfgE.tri_counts[enc_has].sum()/ev_cfgE.n_tri:.1f}% of tokens)")
    lines.append("")

    # --- 2. anneal under space-aware objective
    lines.append("-- search under space-aware objective --")
    ev = evw(WIN)
    best, best_s = None, 1e9
    warm = list(slots_of(ev, WIN))
    for si in range(4):
        if si == 0:
            sp_, ss = warm[:], ev.score(tuple(warm))
        else:
            sp_, ss = ev.anneal(iters=45000, seed=400 + si)
        st, sq = quench(ev, sp_, ss, random.Random(800 + si))
        if sq < best_s:
            best, best_s = st[:], sq
        lay = ev.layout_str(tuple(st))
        s_letters = score_of(Evaluator(ce, dead_of(lay)), lay)
        tag = "warm" if si == 0 else f"r{si}"
        lines.append(f"  {tag}: space {sq:.4f}  letters {s_letters:.4f}  {lay}")
    lay_cand = ev.layout_str(tuple(best))
    s_cand_w, s_cand_l = best_s, score_of(Evaluator(ce, dead_of(lay_cand)), lay_cand)
    lines.append(f"\n  space-aware winner: space {s_cand_w:.4f}  "
                 f"letters {s_cand_l:.4f}  {lay_cand}")

    # --- 3. cross-axis comparison vs cfgE
    s_cfgE_w = score_of(evw(WIN), WIN)
    s_cfgE_l = score_of(Evaluator(ce, dead_of(WIN)), WIN)
    lines.append("")
    lines.append("-- both axes vs cfgE --")
    lines.append(f"  cfgE:           space {s_cfgE_w:.4f}  letters {s_cfgE_l:.4f}")
    lines.append(f"  space winner:   space {s_cand_w:.4f}  letters {s_cand_l:.4f}  "
                 f"(d_space {s_cand_w-s_cfgE_w:+.4f}, "
                 f"d_letters {s_cand_l-s_cfgE_l:+.4f})")
    # candidate under letters-only search too (is it dominated?)
    lines.append("")
    lines.append(f"({time.time()-t_start:.0f}s total)")

    txt = "\n".join(lines) + "\n"
    open("/home/akai/git/layout-opencode-investigation/results/d6_space.txt",
         "w").write(txt)
    print(txt)


if __name__ == "__main__":
    main()
