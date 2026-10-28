#!/usr/bin/env python3
"""D4: load-target penalties (keycraft targets) + balance-weight Pareto.

a) LoadEvaluator = base score + lam * (HLD + FLD + RLD deviations vs
   keycraft load_targets.txt: hands 50/50, fingers 7/10/16/17,
   rows 17.5/75/7.5). Anneal+quench from cfgE; cross-score on original.
b) Re-search at W_BALANCE in {0, 3, 5}; cross-score winners on original.
"""
import random
import sys

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
import search
from scorer import Corpus
from search import Evaluator
from fingers import KEY_ORDER, KEYS, IDEAL_SHARE
from compare import dead_of, slots_of, score_of
from robustness import quench

WIN = ";aiojvlnrwhfpeygtcmsbxquzkd,./"

# keycraft load_targets.txt
T_HAND = (50.0, 50.0)
T_FING = (7.0, 10.0, 16.0, 17.0)   # P R M I, mirrored
T_ROW = (17.5, 75.0, 7.5)          # top home bottom


def load_dev(lfreq_norm, perm, ev):
    """HLD+FLD+RLD relative squared deviation (letters-only usage)."""
    usage_f = {"L": [0.0]*4, "R": [0.0]*4}
    usage_h = {"L": 0.0, "R": 0.0}
    usage_r = [0.0, 0.0, 0.0]
    fidx = {"P": 0, "R": 1, "M": 2, "I": 3}
    for li in range(26):
        k = KEY_ORDER[ev._key_ids(perm)[li]]
        key = KEYS[k]
        w = lfreq_norm[li]
        usage_f[key.hand][fidx[key.alts[0][0]]] += w
        usage_h[key.hand] += w
        row = key.row  # 0 top, 1 home, 2 bottom
        usage_r[row] += w
    dev = 0.0
    tot = sum(usage_h.values())
    for hi, h in enumerate("LR"):
        share = 100.0 * usage_h[h] / tot
        dev += ((share - T_HAND[hi]) / T_HAND[hi]) ** 2
    for fi in range(4):
        share = 100.0 * (usage_f["L"][fi] + usage_f["R"][fi]) / tot
        dev += ((share - T_FING[fi]) / T_FING[fi]) ** 2
    for ri in range(3):
        share = 100.0 * usage_r[ri] / tot
        dev += ((share - T_ROW[ri]) / T_ROW[ri]) ** 2
    return dev


def main():
    ce = Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")
    base_ev = Evaluator(ce, dead_of(WIN))
    lfreq = [ce.L.get(ch, 0.0) for ch in "abcdefghijklmnopqrstuvwxyz"]
    s = sum(lfreq)
    lfreq = [x / s for x in lfreq]

    print("cfgE base load deviation: %.4f" % load_dev(lfreq, slots_of(base_ev, WIN), base_ev))
    lines = ["D4 — load-target penalties + balance Pareto", ""]

    # ---- a) load-penalized search
    for lam in (0.5, 1.5, 3.0):
        ev = Evaluator(ce, dead_of(WIN))

        class LE(Evaluator):
            def score(self, perm):
                return super().score(perm) + lam * load_dev(
                    lfreq, perm, self)

        le = LE(ce, dead_of(WIN))
        warm = list(slots_of(base_ev, WIN))
        best, bs = warm[:], le.score(tuple(warm))
        for si in range(3):
            sp, ss = (warm[:], le.score(tuple(warm))) if si == 0 else \
                le.anneal(iters=45000, seed=1300 + si)
            st, sq = quench(le, sp, ss, random.Random(1400 + si))
            if sq < bs:
                best, bs = st[:], sq
        lay = le.layout_str(tuple(best))
        s_orig = score_of(base_ev, lay)
        dev_cand = load_dev(lfreq, tuple(best), le)
        lines.append(f"lam={lam}: penalized {bs:.4f}  original {s_orig:.4f}  "
                     f"load_dev {dev_cand:.4f}  {lay}")
        print(lines[-1], flush=True)

    # ---- b) balance-weight Pareto re-search
    lines.append("")
    for wb in (0.0, 3.0, 5.0):
        search.W_BALANCE = wb
        ev = Evaluator(ce, dead_of(WIN))  # module global read at score time
        warm = list(slots_of(base_ev, WIN))
        best, bs = warm[:], ev.score(tuple(warm))
        for si in range(3):
            sp, ss = (warm[:], ev.score(tuple(warm))) if si == 0 else \
                ev.anneal(iters=45000, seed=1500 + int(wb*10) + si)
            st, sq = quench(ev, sp, ss, random.Random(1600 + si))
            if sq < bs:
                best, bs = st[:], sq
        search.W_BALANCE = 1.5
        lay = ev.layout_str(tuple(best))
        s_orig = score_of(base_ev, lay)
        lines.append(f"W_BALANCE={wb}: objective {bs:.4f}  original {s_orig:.4f}  {lay}")
        print(lines[-1], flush=True)

    txt = "\n".join(lines) + "\n"
    open("/home/akai/git/layout-opencode-investigation/results/d4_loads.txt", "w").write(txt)
    print(txt)


if __name__ == "__main__":
    main()
