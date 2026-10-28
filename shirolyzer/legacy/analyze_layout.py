"""
analyze_layout.py — Full report for a layout string (30 chars, KEY_ORDER).
Usage: python3 analyze_layout.py "<layout30>" [corpus_dir]
"""
import sys, json
sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from fingers import KEY_ORDER, KEYS, FORDER
from scorer import Corpus, get_tables, get_default_tables
from metrics import category_report, usage_report, print_grid

FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}
LETTERS = "abcdefghijklmnopqrstuvwxyz"


def fingering_guide(layout30, corpus):
    """For each letter: DP-chosen finger distribution over its trigram
    contexts; list top contexts where each alt finger is chosen."""
    m = {ch: KEY_ORDER[i] for i, ch in enumerate(layout30)}
    TRI, TRI_F = get_tables()[1], get_tables()[3]
    Nloc = 31
    # collect per (letter, position-in-trigram, chosen-finger) weighted counts
    from collections import defaultdict
    agg = {ch: defaultdict(float) for ch in LETTERS}
    ctx = {ch: defaultdict(lambda: defaultdict(float)) for ch in LETTERS}
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or any(c not in m for c in tg):
            continue
        ids = [KEY_ORDER.index(m[c]) for c in tg]
        fs = TRI_F[ids[0], ids[1], ids[2]]
        fseq = [FID2[int(x)] for x in fs]
        for pos, ch in enumerate(tg):
            agg[ch][fseq[pos]] += cnt
            ctx[ch][fseq[pos]][tg] += cnt
    guide = {}
    for ch in LETTERS:
        if ch not in m:
            continue
        k = m[ch]
        dflt = KEYS[k].alts[0][0]
        tot = sum(agg[ch].values()) or 1
        entry = {"key": k, "default": dflt, "share": {}}
        for f, c in sorted(agg[ch].items(), key=lambda x: -x[1]):
            entry["share"][f] = c / tot * 100
        # top contexts for each non-default alt
        alts = {}
        for f in entry["share"]:
            if f == dflt:
                continue
            tops = sorted(ctx[ch][f].items(), key=lambda x: -x[1])[:8]
            alts[f] = [(tg, round(c)) for tg, c in tops]
        entry["alt_contexts"] = alts
        guide[ch] = entry
    return guide


def worst_pairs(layout30, corpus, topn=30):
    m = {ch: KEY_ORDER[i] for i, ch in enumerate(layout30)}
    BIG, BIG_F = get_tables()[0], get_tables()[2]
    BIGd = get_default_tables()[0]
    rows = []
    for bg, cnt in corpus.B.items():
        if len(bg) != 2 or bg[0] not in m or bg[1] not in m:
            continue
        i, j = KEY_ORDER.index(m[bg[0]]), KEY_ORDER.index(m[bg[1]])
        packed = int(BIG_F[i, j])
        f1, f2 = FID2[packed // 5], FID2[packed % 5]
        rows.append((BIG[i, j], BIGd[i, j], bg, f1, f2, cnt))
    rows.sort(reverse=True)
    return rows[:topn]


def main():
    layout = sys.argv[1]
    prefix = sys.argv[2] if len(sys.argv) > 2 else "/home/akai/git/layout-opencode-investigation/corpus/"
    assert len(layout) == 30, len(layout)
    corpus = Corpus.load(prefix)
    print_grid(layout)
    print()
    rep = category_report(layout, corpus)
    keys = ["sfb", "sfb_default_pct", "sfb_unavoidable_pct", "scissor_IM_hsb",
            "scissor_IM_fsb", "scissor_MR", "scissor_RP", "roll_in", "roll_out",
            "leap", "alt", "repeat", "balance"]
    print("== bigram categories (% of letter bigrams, alt-optimized) ==")
    for k in ["sfb", "scissor_IM_hsb", "scissor_IM_fsb", "scissor_MR", "scissor_RP",
              "roll_in", "roll_out", "leap", "alt", "repeat"]:
        print(f"  {k:16s} {rep.get(k, 0):6.2f}")
    print(f"  SFB (default fingering only): {rep['sfb_default_pct']:.2f}%")
    print(f"  SFB unavoidable even w/alts : {rep['sfb_unavoidable_pct']:.2f}%")
    print("\n== worst bigrams (cost under alts | cost default | fingering) ==")
    for cost, bg, f1, f2, cnt in rep["worst_bigrams"][:20]:
        print(f"  {bg}  n={cnt:7d}  alts={cost:5.2f} ({f1}{f2})")
    # scissors with tolerance labels (IM retracted-middle direction
    # = prefer-avoid (HSB) / intolerant (FSB); MR & RP all intolerant)
    from fingers import scissor_cost, cheb
    m = {ch: KEY_ORDER[i] for i, ch in enumerate(layout)}
    sc_rows = []
    for bg, cnt in corpus.B.items():
        if len(bg) != 2 or bg[0] not in m or bg[1] not in m:
            continue
        n1, n2 = m[bg[0]], m[bg[1]]
        i, j = KEY_ORDER.index(n1), KEY_ORDER.index(n2)
        packed = int(get_tables()[2][i, j])
        f1, f2 = FID2[packed // 5], FID2[packed % 5]
        c = scissor_cost(n1, f1, n2, f2)
        if c > 0:
            dist = cheb((KEYS[n1].row, KEYS[n1].col), (KEYS[n2].row, KEYS[n2].col))
            outer = f1 if FORDER[f1] < FORDER[f2] else f2
            inner = f2 if outer == f1 else f1
            if (outer, inner) == ("M", "I"):
                kind = "IM-HSB(prefer-avoid)" if dist == 1 else "IM-FSB(INTOLERANT)"
            else:
                kind = f"{outer}-{inner}(INTOLERANT)"
            sc_rows.append((c * cnt, c, kind, bg, f1, f2, cnt))
    sc_rows.sort(reverse=True)
    print("\n== top scissors (weight=cost*n | user tolerance) ==")
    for w, c, kind, bg, f1, f2, cnt in sc_rows[:15]:
        print(f"  {bg}  n={cnt:6d} cost={c:4.2f} {kind:22s} ({f1}{f2})")
    tot_sc = sum(r[0] for r in sc_rows)
    print(f"  total scissor weight {tot_sc:.0f} over {len(sc_rows)} bigram types")
    print("\n== usage (default fingers x letter freq) ==")
    u = usage_report(layout, corpus)
    for k in sorted(u):
        print(f"  {k}: {u[k]:5.2f}%", end="")
    print()
    guide = fingering_guide(layout, corpus)
    out = {"layout": layout, "categories": {k: rep[k] for k in rep if k != "worst_bigrams"
                                            and k != "per_bigram"},
           "worst_bigrams": [[c, bg, f1, f2, n] for c, _, bg, f1, f2, n in worst_pairs(layout, corpus)],
           "usage": u, "guide": guide}
    json.dump(out, open("/home/akai/git/layout-opencode-investigation/results/analysis.json", "w"), indent=1)
    print("\nwrote results/analysis.json")


if __name__ == "__main__":
    main()
