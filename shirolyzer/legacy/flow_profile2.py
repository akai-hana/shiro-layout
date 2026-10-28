#!/usr/bin/env python3
"""flow_profile2.py — bigram/trigram class profile under the scorer2 model.

Same category definitions as metrics.py / flow_audit.py, but driven by the
round-6b scorer2 tables. Bigram classes use the context-optimal fingering
(BIG_F); trigram patterns use the default finger of each key, matching the
flow_profile_r6 convention.

Writes results/flow_profile_r7.txt.
"""
import sys

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from fingers import KEYS, KEY_ORDER, FORDER
from scorer2 import Corpus, get_tables

CORPUS = "/home/akai/git/layout-opencode-investigation/corpus/"
OUT = "/home/akai/git/layout-opencode-investigation/results/flow_profile_r7.txt"

CFG_E = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
OLD_BEST = ";stckqdaiwnlprhyeguomfvbzxj,./"
NEW_BEST = ";stpbqdaiynhcrmkeguowvflzxj,./"
ANNEAL_BEST = ";stckzdaiwnlprhyeguobfvmjqx,./"

FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}

BIGRAM_CATS = ("repeat", "sfb", "leap", "roll_in", "roll_out", "alt")
TRIGRAM_CATS = ("alternate", "inroll", "outroll", "onehand", "redirect",
                "redirect_weak", "sfb_tri", "other")


def key_of(ch, lay):
    return KEY_ORDER[lay.index(ch)]


def bg_category(lay, a, b, BIG_F):
    n1, n2 = key_of(a, lay), key_of(b, lay)
    i, j = KEY_ORDER.index(n1), KEY_ORDER.index(n2)
    p = int(BIG_F[i, j])
    f1, f2 = FID2[p // 5], FID2[p % 5]
    if n1 == n2:
        return "repeat"
    if KEYS[n1].hand == KEYS[n2].hand and f1 == f2:
        return "sfb"
    if KEYS[n1].hand != KEYS[n2].hand:
        return "alt"
    o1, o2 = FORDER[f1], FORDER[f2]
    dcol = abs(KEYS[n1].col - KEYS[n2].col)
    if abs(o2 - o1) >= 2 or dcol >= 3:
        return "leap"
    return "roll_in" if o2 > o1 else "roll_out"


def tg_category(lay, tg):
    n1, n2, n3 = (key_of(c, lay) for c in tg)
    ks = (KEYS[n1], KEYS[n2], KEYS[n3])
    hs = tuple(k.hand for k in ks)
    fs = tuple(KEYS[n].alts[0][0] for n in (n1, n2, n3))
    order = tuple(FORDER[f] for f in fs)
    if hs in (("L", "R", "L"), ("R", "L", "R")):
        return "alternate"
    if hs[0] == hs[1] == hs[2]:
        if fs[0] == fs[1] or fs[1] == fs[2]:
            return "sfb_tri"
        d1, d2 = order[1] - order[0], order[2] - order[1]
        if d1 * d2 < 0:
            return "redirect" if "I" in fs else "redirect_weak"
        return "onehand"
    if hs[0] == hs[1]:
        if fs[0] == fs[1]:
            return "sfb_tri"
        return "inroll" if order[0] < order[1] else "outroll"
    if hs[1] == hs[2]:
        if fs[1] == fs[2]:
            return "sfb_tri"
        return "inroll" if order[1] < order[2] else "outroll"
    return "other"


def profile(corpus, lay, BIG_F):
    bc = {k: 0.0 for k in BIGRAM_CATS}
    bt = 0.0
    for tg, cnt in corpus.B.items():
        if len(tg) != 2 or any(ch not in lay for ch in tg):
            continue
        bt += cnt
        bc[bg_category(lay, tg[0], tg[1], BIG_F)] += cnt
    tc = {k: 0.0 for k in TRIGRAM_CATS}
    tt = 0.0
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or any(ch not in lay for ch in tg):
            continue
        tt += cnt
        tc[tg_category(lay, tg)] += cnt
    return bt, bc, tt, tc


def main():
    corpus = Corpus.load(CORPUS)
    _, _, BIG_F, _ = get_tables()
    layouts = [("cfgE", CFG_E), ("OLD_BEST", OLD_BEST),
               ("NEW_BEST", NEW_BEST), ("annealBEST", ANNEAL_BEST)]
    lines = ["Round-7 flow profile (scorer2, round-6b model)",
             "bigrams: context-optimal fingering; trigrams: default fingers",
             ""]
    for name, lay in layouts:
        bt, bc, tt, tc = profile(corpus, lay, BIG_F)
        lines.append(f"=== {name} {lay} ===")
        for cat in BIGRAM_CATS:
            lines.append(f"  {cat:10s} {100.0 * bc[cat] / bt:6.2f}%")
        lines.append(f"  {'rolls':10s} "
                     f"{100.0 * (bc['roll_in'] + bc['roll_out']) / bt:6.2f}%")
        lines.append("  trigram (default fingers):")
        for cat in TRIGRAM_CATS:
            lines.append(f"    {cat:13s} {100.0 * tc[cat] / tt:6.2f}%")
        lines.append("")
    txt = "\n".join(lines) + "\n"
    with open(OUT, "w") as f:
        f.write(txt)
    print(txt, end="")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
