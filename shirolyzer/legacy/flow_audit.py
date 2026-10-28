#!/usr/bin/env python3
"""flow_audit.py — categorize every bigram/trigram + top-250/1000 words.

Categories use the SAME definitions as metrics.py (context-optimal fingering
from the BIG/TRI tables, scissor_cost, FORDER rolls):
  bigram: repeat | sfb | leap | roll_in | roll_out | alt
          + scissors by pair (IM_hsb, IM_fsb, MR, RP)
  trigram: alternate | inroll | outroll | onehand | redirect | redirect_weak
           | sfb_tri | other

Words: top-250 (monkeytype-200 + cyanophage words-english) and top-1000
(monkeytype-1k), weighted by real frequency. Per word + aggregate.

Outputs results/flow_audit.txt.
"""
import json
import sys

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from fingers import KEYS, KEY_ORDER, FORDER, scissor_cost, cheb
from scorer import Corpus, get_tables

CFGE = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
CORPUS = "/home/akai/git/layout-opencode-investigation/corpus/"
CYANO = "/tmp/opencode/analyzers/cyanophage/words-english.json"
MT200 = "/tmp/opencode/analyzers/200-analyzer/wordlists/monkeytype-200.json"
MT1K = "/tmp/opencode/analyzers/200-analyzer/wordlists/monkeytype-1k.json"
OUT = "/home/akai/git/layout-opencode-investigation/results/flow_audit.txt"

FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}


def key_of(ch, lay):
    return KEY_ORDER[lay.index(ch)]


def bg_category(lay, BIG_F, a, b):
    """Return (category, f1, f2, scissor) for bigram ab under chosen fingering."""
    n1, n2 = key_of(a, lay), key_of(b, lay)
    i, j = KEY_ORDER.index(n1), KEY_ORDER.index(n2)
    packed = int(BIG_F[i, j])
    f1, f2 = FID2[packed // 5], FID2[packed % 5]
    sc = scissor_cost(n1, f1, n2, f2)
    sc_name = None
    if sc > 0:
        dist = cheb((KEYS[n1].row, KEYS[n1].col), (KEYS[n2].row, KEYS[n2].col))
        outer = f1 if FORDER[f1] < FORDER[f2] else f2
        inner = f2 if outer == f1 else f1
        if (outer, inner) == ("M", "I"):
            sc_name = "IM_hsb" if dist == 1 else "IM_fsb"
        elif (outer, inner) == ("R", "M"):
            sc_name = "MR"
        elif (outer, inner) == ("P", "R"):
            sc_name = "RP"
    if n1 == n2:
        return "repeat", f1, f2, sc_name
    if KEYS[n1].hand == KEYS[n2].hand and f1 == f2:
        return "sfb", f1, f2, sc_name
    if KEYS[n1].hand != KEYS[n2].hand:
        return "alt", f1, f2, sc_name
    o1, o2 = FORDER[f1], FORDER[f2]
    dcol = abs(KEYS[n1].col - KEYS[n2].col)
    if abs(o2 - o1) >= 2 or dcol >= 3:
        return "leap", f1, f2, sc_name
    if o2 > o1:
        return "roll_in", f1, f2, sc_name
    return "roll_out", f1, f2, sc_name


def tg_category(lay, a, b, c):
    """Trigram pattern under DEFAULT fingers (metrics.py convention)."""
    n1, n2, n3 = key_of(a, lay), key_of(b, lay), key_of(c, lay)
    ks = (KEYS[n1], KEYS[n2], KEYS[n3])
    hs = tuple(k.hand for k in ks)
    fs = tuple(KEYS[n].alts[0][0] for n in (n1, n2, n3))
    os_ = tuple(FORDER[f] for f in fs)
    if hs in (("L", "R", "L"), ("R", "L", "R")):
        return "alternate", fs
    if hs[0] == hs[1] == hs[2]:
        if fs[0] == fs[1] or fs[1] == fs[2]:
            return "sfb_tri", fs
        d1, d2 = os_[1] - os_[0], os_[2] - os_[1]
        if d1 * d2 < 0:
            return ("redirect" if "I" in fs else "redirect_weak"), fs
        return "onehand", fs
    if hs[0] == hs[1]:
        if fs[0] == fs[1]:
            return "sfb_tri", fs
        return ("inroll" if os_[0] < os_[1] else "outroll"), fs
    if hs[1] == hs[2]:
        if fs[1] == fs[2]:
            return "sfb_tri", fs
        return ("inroll" if os_[1] < os_[2] else "outroll"), fs
    return "other", fs


def main():
    ce = Corpus.load(CORPUS)
    BIG, TRI, BIG_F, TRI_F = get_tables()
    lay = CFGE

    # ---------- bigrams ----------
    bgs = [(cnt, bg) for bg, cnt in ce.B.items()
           if len(bg) == 2 and bg.isalpha() and bg.islower()]
    bgs.sort(reverse=True)
    bg_tot = sum(cnt for cnt, _ in bgs)
    lines = []
    A = lines.append
    A("FLOW AUDIT — cfgE `;aiojvlnrwhfpeygtcmsbxquzkd,./`")
    A("bigram/trigram categories: context-optimal fingering (BIG_F)")
    A("trigram patterns: DEFAULT fingering (metrics.py convention)")
    A("")
    A(f"== ALL {len(bgs)} BIGRAMS (most->least common) ==")
    A(f"{'#':>4} {'bg':4s} {'count':>8} {'freq%':>6} {'category':9s} "
      f"{'fing':4s} {'scissor':7s}")
    cat_tot, sc_tot = {}, {}
    for rank, (cnt, bg) in enumerate(bgs, 1):
        cat, f1, f2, sc = bg_category(lay, BIG_F, bg[0], bg[1])
        freq = 100 * cnt / bg_tot
        cat_tot[cat] = cat_tot.get(cat, 0) + cnt
        if sc:
            sc_tot[sc] = sc_tot.get(sc, 0) + cnt
        A(f"{rank:4d} {bg:4s} {cnt:8d} {freq:6.3f} {cat:9s} "
          f"{f1+f2:4s} {sc or '':7s}")
    tot = bg_tot
    A("")
    A("bigram category totals (% of all bigram mass):")
    for cat, c in sorted(cat_tot.items(), key=lambda x: -x[1]):
        A(f"  {cat:9s} {100*c/tot:6.2f}%")
    A("scissors (subset of rolls/leaps, by pair):")
    for sc, c in sorted(sc_tot.items(), key=lambda x: -x[1]):
        A(f"  {sc:7s} {100*c/tot:6.3f}%")
    A("")

    # ---------- trigrams ----------
    tgs = [(cnt, tg) for tg, cnt in ce.T.items()
           if len(tg) == 3 and tg.isalpha() and tg.islower()]
    tgs.sort(reverse=True)
    ttot = sum(cnt for cnt, _ in tgs)
    A(f"== TOP 200 of {len(tgs)} TRIGRAMS (most->least common) ==")
    A(f"{'#':>4} {'tg':4s} {'count':>8} {'freq%':>6} {'pattern':14s} fing")
    tg_cat = {}
    for rank, (cnt, tg) in enumerate(tgs, 1):
        pat, fs = tg_category(lay, tg[0], tg[1], tg[2])
        tg_cat[pat] = tg_cat.get(pat, 0) + cnt
        if rank <= 200:
            A(f"{rank:4d} {tg:4s} {cnt:8d} "
              f"{100*cnt/ttot:6.3f} {pat:14s} "
              f"{fs[0]}{fs[1]}{fs[2]}")
    A("")
    A("trigram pattern totals (% of all trigram mass, all "
      f"{len(tgs)} types):")
    for pat, c in sorted(tg_cat.items(), key=lambda x: -x[1]):
        A(f"  {pat:14s} {100*c/ttot:6.2f}%")
    A("")

    # ---------- words ----------
    cyano = json.load(open(CYANO))
    mt200 = json.load(open(MT200))
    mt1k = json.load(open(MT1K))

    def norm_list(src):
        # cyanophage: dict word->freq.  monkeytype: {"texts":[ordered words]}
        if isinstance(src, dict) and "texts" in src:
            src = src["texts"]
        if isinstance(src, dict):
            items = [(w.lower(), float(f)) for w, f in src.items()
                     if w.isalpha() and w.islower()]
            items.sort(key=lambda x: -x[1])
            return items
        items = [(w.lower(), float(len(src) - i)) for i, w in enumerate(src)
                 if isinstance(w, str) and w.isalpha() and w.islower()]
        return items

    def analyze_words(items, label, topn):
        items = items[:topn]
        A(f"== TOP {len(items)} WORDS — {label} ==")
        A("per-word bigram/trigram categories (default-fingering patterns)")
        A(f"{'#':>4} {'word':12s} {'wfreq%':>6}  bigrams  |  trigrams")
        cat_w, tg_w = {}, {}
        bg_inst = tg_inst = 0.0
        for rank, (w, f) in enumerate(items, 1):
            bgs_w = [bg_category(lay, BIG_F, w[i], w[i + 1])[0]
                     for i in range(len(w) - 1)]
            tgs_w = [tg_category(lay, w[i], w[i + 1], w[i + 2])[0]
                     for i in range(len(w) - 2)]
            bg_inst += f * len(bgs_w)
            tg_inst += f * len(tgs_w)
            for c_ in bgs_w:
                cat_w[c_] = cat_w.get(c_, 0) + f
            for p in tgs_w:
                tg_w[p] = tg_w.get(p, 0) + f
            A(f"{rank:4d} {w:12s} {100*f/sum(x[1] for x in items):6.2f}  "
              f"{' '.join(bgs_w):32s} | {' '.join(tgs_w)}")
        A("")
        A(f"aggregate bigram categories over top-{len(items)} words "
          f"(freq-weighted, {bg_inst:.0f} bigram-instances):")
        for cat, c_ in sorted(cat_w.items(), key=lambda x: -x[1]):
            A(f"  {cat:9s} {100*c_/bg_inst:6.2f}%")
        A(f"aggregate trigram patterns over top-{len(items)} words "
          f"({tg_inst:.0f} trigram-instances):")
        for pat, c_ in sorted(tg_w.items(), key=lambda x: -x[1]):
            A(f"  {pat:14s} {100*c_/tg_inst:6.2f}%")
        A("")

    analyze_words(norm_list(mt200), "monkeytype-200", 250)
    analyze_words(norm_list(mt1k), "monkeytype-1k", 1000)

    txt = "\n".join(lines) + "\n"
    open(OUT, "w").write(txt)
    print(f"wrote {OUT} ({len(lines)} lines)")
    # print the aggregate summaries to stdout for the chat
    print("\n".join(lines[:20]))
    print("...")
    # print category summaries
    for i, ln in enumerate(lines):
        if "category totals" in ln or "pattern totals" in ln or \
                "aggregate" in ln:
            print("\n".join(lines[i:i + 12]))


if __name__ == "__main__":
    main()
