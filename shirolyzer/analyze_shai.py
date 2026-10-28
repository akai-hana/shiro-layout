#!/usr/bin/env python3
"""Diagnostics for the calibrated Shai-primary layout (scorer2 tables)."""
import json
import sys
from collections import defaultdict

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from compare import dead_of, slots_of
from fingers import FORDER, KEYS, KEY_ORDER, cheb, scissor_cost
from objective import (DEFAULT_PRIMARY_SACRIFICE, PRIMARY_PATH,
                       SECONDARY_PATH, build_composite, load_primary,
                       load_secondary)
from scorer2 import (Corpus, FID, get_default_tables, get_tables,
                     _alt_context_penalty)
from search2 import Evaluator, LETTERS

ROOT = "/home/akai/git/layout-opencode-investigation/"
BEST = ";tspbxgoiwndhrlyecuafkvmzjq,./"
FID2 = {v: k for k, v in FID.items()}


def keymap(lay):
    return {ch: KEY_ORDER[i] for i, ch in enumerate(lay) if ch in LETTERS}


def category_report(lay, corpus):
    m = keymap(lay)
    BIG, TRI, BIG_F, TRI_F = get_tables()
    stats = defaultdict(float)
    total = 0.0
    worst = []
    scissors = []
    for bg, cnt in corpus.B.items():
        if len(bg) != 2 or bg[0] not in m or bg[1] not in m:
            continue
        n1, n2 = m[bg[0]], m[bg[1]]
        i, j = KEY_ORDER.index(n1), KEY_ORDER.index(n2)
        p = int(BIG_F[i, j])
        f1, f2 = FID2[p // 5], FID2[p % 5]
        cost = BIG[i, j]
        d1, d2 = KEYS[n1].alts[0][0], KEYS[n2].alts[0][0]
        total += cnt
        worst.append((cost, bg, f1, f2, cnt))
        if n1 == n2:
            stats["repeat"] += cnt
        elif KEYS[n1].hand == KEYS[n2].hand and f1 == f2:
            stats["sfb"] += cnt
        elif KEYS[n1].hand != KEYS[n2].hand:
            stats["alt"] += cnt
        else:
            o1, o2 = FORDER[f1], FORDER[f2]
            dcol = abs(KEYS[n1].col - KEYS[n2].col)
            if abs(o2 - o1) >= 2 or dcol >= 3:
                stats["leap"] += cnt
            elif o2 > o1:
                stats["roll_in"] += cnt
            else:
                stats["roll_out"] += cnt
            sc = scissor_cost(n1, f1, n2, f2)
            if sc > 0:
                dist = cheb((KEYS[n1].row, KEYS[n1].col),
                            (KEYS[n2].row, KEYS[n2].col))
                outer = f1 if FORDER[f1] < FORDER[f2] else f2
                inner = f2 if outer == f1 else f1
                if (outer, inner) == ("M", "I"):
                    stats["scissor_IM_hsb" if dist == 1 else "scissor_IM_fsb"] += cnt
                elif (outer, inner) == ("R", "M"):
                    stats["scissor_MR"] += cnt
                elif (outer, inner) == ("P", "R"):
                    stats["scissor_RP"] += cnt
                scissors.append((sc * cnt, sc, bg, f1, f2, cnt,
                                 outer + inner))
        if (n1 != n2 and d1 == d2 and KEYS[n1].hand == KEYS[n2].hand):
            stats["sfb_default"] += cnt
        if n1 != n2 and f1 == f2 and KEYS[n1].hand == KEYS[n2].hand:
            stats["sfb_chosen"] += cnt
    total = total or 1.0
    cats = {k: stats[k] / total * 100 for k in [
        "sfb", "sfb_default", "sfb_chosen", "scissor_IM_hsb",
        "scissor_IM_fsb", "scissor_MR", "scissor_RP", "roll_in",
        "roll_out", "leap", "alt", "repeat"]}
    cats["total_tokens"] = total
    worst.sort(reverse=True)
    scissors.sort(reverse=True)
    return cats, worst[:30], scissors[:20]


def trigram_patterns(lay, corpus):
    m = keymap(lay)
    TRI_F = get_tables()[3]
    acc = defaultdict(float)
    total = 0.0
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or any(c not in m for c in tg):
            continue
        ks = [m[c] for c in tg]
        ids = [KEY_ORDER.index(k) for k in ks]
        fs = tuple(FID2[int(x)] for x in TRI_F[ids[0], ids[1], ids[2]])
        hs = tuple(KEYS[k].hand for k in ks)
        os_ = tuple(FORDER[f] for f in fs)
        total += cnt
        if hs in (("L", "R", "L"), ("R", "L", "R")):
            acc["alternate"] += cnt
        elif hs[0] == hs[1] == hs[2]:
            if fs[0] == fs[1] or fs[1] == fs[2]:
                acc["sfb_tri"] += cnt
            elif (os_[1] - os_[0]) * (os_[2] - os_[1]) < 0:
                acc["redirect" if "I" in fs else "redirect_weak"] += cnt
            else:
                acc["onehand"] += cnt
        elif hs[0] == hs[1]:
            if fs[0] == fs[1]:
                acc["sfb_tri"] += cnt
            elif os_[0] < os_[1]:
                acc["inroll"] += cnt
            else:
                acc["outroll"] += cnt
        elif hs[1] == hs[2]:
            if fs[1] == fs[2]:
                acc["sfb_tri"] += cnt
            elif os_[1] < os_[2]:
                acc["inroll"] += cnt
            else:
                acc["outroll"] += cnt
        else:
            acc["other"] += cnt
    total = total or 1.0
    out = {k: acc[k] / total * 100 for k in acc}
    out["total_rolls"] = out["inroll"] + out["outroll"]
    out["total_redirects"] = out["redirect"] + out["redirect_weak"]
    return out


def finger_usage(lay, corpus, actual=True):
    m = keymap(lay)
    usage = {h: {f: 0.0 for f in "PRMI"} for h in "LR"}
    if actual:
        TRI_F = get_tables()[3]
        for tg, cnt in corpus.T.items():
            if len(tg) != 3 or any(c not in m for c in tg):
                continue
            ks = [m[c] for c in tg]
            ids = [KEY_ORDER.index(k) for k in ks]
            fs = TRI_F[ids[0], ids[1], ids[2]]
            for k, fid in zip(ks, fs):
                usage[KEYS[k].hand][FID2[int(fid)]] += cnt
    else:
        for ch, cnt in corpus.L.items():
            if ch not in m:
                continue
            k = m[ch]
            usage[KEYS[k].hand][KEYS[k].alts[0][0]] += cnt
    tot = sum(sum(usage[h].values()) for h in "LR") or 1
    out = {}
    for h in "LR":
        for f in "PRMI":
            out[f"{h}{f}"] = usage[h][f] / tot * 100
    out["handL"] = sum(usage["L"].values()) / tot * 100
    out["handR"] = sum(usage["R"].values()) / tot * 100
    return out


def fingering_guide(lay, corpus):
    m = keymap(lay)
    TRI_F = get_tables()[3]
    agg = {ch: defaultdict(float) for ch in LETTERS}
    ctx = {ch: defaultdict(lambda: defaultdict(float)) for ch in LETTERS}
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or any(c not in m for c in tg):
            continue
        ids = [KEY_ORDER.index(m[c]) for c in tg]
        fs = [FID2[int(x)] for x in TRI_F[ids[0], ids[1], ids[2]]]
        for pos, ch in enumerate(tg):
            agg[ch][fs[pos]] += cnt
            ctx[ch][fs[pos]][tg] += cnt
    guide = {}
    for ch in LETTERS:
        if ch not in m:
            continue
        k = m[ch]
        dflt = KEYS[k].alts[0][0]
        tot = sum(agg[ch].values()) or 1
        entry = {"key": k, "default": dflt,
                 "share": {f: c / tot * 100 for f, c in
                           sorted(agg[ch].items(), key=lambda x: -x[1])},
                 "alt_contexts": {}}
        for f in entry["share"]:
            if f == dflt:
                continue
            entry["alt_contexts"][f] = [
                (tg, round(c)) for tg, c in
                sorted(ctx[ch][f].items(), key=lambda x: -x[1])[:8]]
        guide[ch] = entry
    return guide


def main():
    lay = sys.argv[1] if len(sys.argv) > 1 else BEST
    if len(lay) != 30:
        sys.exit("layout must be 30 chars")
    corpus = Corpus.load(ROOT + "corpus/shai/")
    primary = load_primary(PRIMARY_PATH)
    secondary = load_secondary(SECONDARY_PATH)
    composite, pc, sc, scale = build_composite(
        primary=primary, secondary=secondary,
        sacrifice=DEFAULT_PRIMARY_SACRIFICE)
    def score(c):
        ev = Evaluator(c, dead_of(lay))
        return ev.score(slots_of(ev, lay))
    comp, pri, sec = score(composite), score(pc), score(sc)
    cats, worst, scissors = category_report(lay, corpus)
    pats = trigram_patterns(lay, corpus)
    use_actual = finger_usage(lay, corpus, actual=True)
    use_default = finger_usage(lay, corpus, actual=False)
    guide = fingering_guide(lay, corpus)

    lines = []
    def out(s=""):
        print(s)
        lines.append(s)
    out(f"layout: {lay}")
    out("grid (QWERTY order, left/right split 5+5):")
    for r in range(3):
        row = lay[r * 10:(r + 1) * 10]
        out(f"  {row[:5]} | {row[5:]}")
    out(f"composite={comp:.9f} primary={pri:.9f} spanish={sec:.9f}")
    out()
    out("== bigram categories (% of letter bigrams, chosen fingering) ==")
    for k in ["sfb", "scissor_IM_hsb", "scissor_IM_fsb", "scissor_MR",
              "scissor_RP", "roll_in", "roll_out", "leap", "alt", "repeat"]:
        out(f"  {k:16s} {cats[k]:6.2f}")
    out(f"  SFB (default fingers only): {cats['sfb_default']:.2f}%")
    out(f"  SFB (chosen fingers):       {cats['sfb_chosen']:.2f}%")
    out()
    out("== worst bigrams (chosen cost, fingering, count) ==")
    for cost, bg, f1, f2, cnt in worst[:20]:
        out(f"  {bg}  n={cnt:8d}  {cost:6.2f}  {f1}{f2}")
    out()
    out("== scissors (weight=cost*n, kind, fingering) ==")
    for w, c, bg, f1, f2, cnt, pair in scissors[:15]:
        out(f"  {bg}  n={cnt:8d}  cost={c:5.2f}  {pair:3s}  {f1}{f2}")
    out()
    out("== trigram patterns (chosen fingering, % of trigrams) ==")
    for k in ["alternate", "inroll", "outroll", "total_rolls", "onehand",
              "redirect", "redirect_weak", "total_redirects", "sfb_tri",
              "other"]:
        out(f"  {k:16s} {pats.get(k, 0):6.2f}")
    out()
    out("== finger load (% of all strokes) ==")
    out("  actual chosen fingers:")
    for h in "LR":
        out(f"    {h}: " + "  ".join(f"{f}={use_actual[h+f]:5.2f}%" for f in "PRMI"))
    out("  default-finger letter load:")
    for h in "LR":
        out(f"    {h}: " + "  ".join(f"{f}={use_default[h+f]:5.2f}%" for f in "PRMI"))
    out()
    out("== alpha fingering guide (actual trigram DP) ==")
    for ch in LETTERS:
        g = guide[ch]
        shares = " ".join(f"{f}={v:.1f}%" for f, v in g["share"].items())
        out(f"  {ch}@{g['key']:<2s} default {g['default']}  {shares}")
        for f, tops in g["alt_contexts"].items():
            ex = ", ".join(f"{tg}:{n}" for tg, n in tops[:5])
            out(f"      alt {f}: {ex}")
    out()
    out(f"wrote {ROOT}results/round8/analysis_shai.txt and .json")
    out_path = ROOT + "results/round8/analysis_shai.txt"
    open(out_path, "w").write("\n".join(lines) + "\n")
    json.dump({"layout": lay,
               "scores": {"composite": comp, "primary": pri, "spanish": sec},
               "categories": cats, "worst_bigrams":
                   [[c, bg, f1, f2, n] for c, bg, f1, f2, n in worst],
               "scissors": [[w, c, bg, f1, f2, n, p] for
                            w, c, bg, f1, f2, n, p in scissors],
               "trigram_patterns": pats,
               "usage_actual": use_actual,
               "usage_default": use_default,
               "guide": guide},
              open(ROOT + "results/round8/analysis_shai.json", "w"), indent=1)


if __name__ == "__main__":
    main()
