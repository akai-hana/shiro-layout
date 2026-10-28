#!/usr/bin/env python3
"""B8: hybrid EN+ES side layout — blended-corpus annealing + Pareto curve.

English stays primary: cfgE is untouched; this produces CANDIDATE side
layouts for bilingual use, each measured on both pure axes (pure-English
Norvig objective, pure-Spanish objective).

Blend definition: per-corpus rates (counts normalized to 1e6 mass) mixed as
(1-w)*EN + w*ES for L, B and T independently; anneal fresh under the blend
(warm start cfgE + cfgG perms + one random), quench, then score the result
pure-EN and pure-ES.

w = 0 control must reproduce ~cfgE. Undead chars: same dead set as cfgE
(',','.', ';','/', '/'); apostrophe off-grid; Spanish enclitics/accents/ñ are
not in the 30-key alphabet and are excluded from the ES corpus as built.
"""
import json
import random
import sys
import time

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer import Corpus
from search import Evaluator
from compare import dead_of, slots_of, score_of, REF
from robustness import quench

EN_DIR = "/home/akai/git/layout-opencode-investigation/corpus/"
ES_DIR = "/home/akai/git/layout-opencode-investigation/corpus/spanish/"
OUT_TXT = "/home/akai/git/layout-opencode-investigation/results/hybrid_pareto.txt"
OUT_JSON = "/home/akai/git/layout-opencode-investigation/results/hybrid_pareto.json"

CFG_E = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
CFG_G = "faiojwlrn;hgpeyvtcmsbqxuzkd,./"
GRAPHITE = "bldwz'foujnrtsgyhaeiqxmcvkp.-/"
REFS = {
    "cfgE (EN primary)": CFG_E,
    "cfgG": CFG_G,
    "Sturdy": REF["Sturdy"],
    "Graphite": GRAPHITE,
    "Gallium": REF["Gallium"],
    "Colemak-DH": REF["Colemak-DH"],
    "Canary": REF["Canary"],
    "Semimak": REF["Semimak"],
    "QWERTY": REF["QWERTY"],
}
BLEND_WS = [0.0, 0.2, 0.35, 0.5, 0.65, 0.8]


def norm(d):
    tot = float(sum(d.values()))
    return {k: v * 1e6 / tot for k, v in d.items()}


def blended(en_dir, es_dir, w):
    def load(d):
        import json as j
        return (j.load(open(d + "letter_freq.json")),
                j.load(open(d + "bigram_freq.json")),
                j.load(open(d + "trigram_freq.json")))
    le, be, te = load(en_dir)
    ls, bs, ts = load(es_dir)
    le, be, te, ls, bs, ts = map(norm, (le, be, te, ls, bs, ts))

    def mix(a, b):
        out = {}
        for k in set(a) | set(b):
            v = (1 - w) * a.get(k, 0.0) + w * b.get(k, 0.0)
            if v > 1e-9:
                out[k] = v
        return out
    return Corpus(mix(le, ls), mix(be, bs), mix(te, ts))


def pure_axes(layout):
    """score layout on pure EN and pure ES corpora (per-layout dead set)."""
    cen = Corpus.load(EN_DIR)
    ces = Corpus.load(ES_DIR)
    s_en = score_of(Evaluator(cen, dead_of(layout)), layout)
    s_es = score_of(Evaluator(ces, dead_of(layout)), layout)
    return s_en, s_es


def main():
    t_start = time.time()
    cen = Corpus.load(EN_DIR)
    ces = Corpus.load(ES_DIR)
    # pre-built pure evaluators for final scoring (per layout string)
    lines = ["B8 — hybrid EN+ES blended search (side layouts; EN primary)",
             f"blend = (1-w)*EN + w*ES on normalized L/B/T", ""]
    results = []

    for w in BLEND_WS:
        t0 = time.time()
        corpus = blended(EN_DIR, ES_DIR, w)
        ev = Evaluator(corpus, dead_of(CFG_E))
        seeds = [slots_of(ev, CFG_E)]
        best, best_s = None, 1e9
        for si in range(6):
            if si == 0:
                sp, ss = seeds[0], ev.score(seeds[0])
            else:
                sp, ss = ev.anneal(iters=45000, seed=200 + int(w * 100) + si)
            st, sq = quench(ev, list(sp), ss, random.Random(300 + si))
            if sq < best_s:
                best, best_s = st[:], sq
        lay = ev.layout_str(tuple(best))
        s_en, s_es = pure_axes(lay)
        results.append({"w": w, "layout": lay, "blend_score": best_s,
                        "EN": s_en, "ES": s_es})
        row = (f"w={w:<5}  EN {s_en:6.3f}  ES {s_es:6.3f}  "
               f"({time.time()-t0:.0f}s)  {lay}")
        print(row, flush=True)
        lines.append(row)

    # Pareto view vs known ref points
    lines.append("")
    lines.append("--- Pareto view (EN score, ES score; lower both better)")
    base = results[0]
    ref_pts = {}
    for name, lay in REFS.items():
        s_en, s_es = pure_axes(lay)
        ref_pts[name] = (s_en, s_es)
        lines.append(f"  ref {name:20s} EN {s_en:6.3f}  ES {s_es:6.3f}")
    lines.append("")
    lines.append(f"{'w':>6s} {'EN':>7s} {'dEN':>7s} {'ES':>7s} {'dES':>7s}  "
                 f"layout")
    for r in results:
        d_en = r["EN"] - base["EN"]
        d_es = r["ES"] - base["ES"]
        lines.append(f"{r['w']:6.2f} {r['EN']:7.3f} {d_en:+7.3f} "
                     f"{r['ES']:7.3f} {d_es:+7.3f}  {r['layout']}")

    lines.append("")
    lines.append("--- Pareto frontier among blended candidates")
    for r in results:
        dominated = any(
            (o["EN"] <= r["EN"] and o["ES"] <= r["ES"]
             and (o["EN"] < r["EN"] or o["ES"] < r["ES"]))
            for o in results if o is not r)
        r["dominated"] = dominated
        tag = "DOMINATED" if dominated else "frontier"
        lines.append(f"  w={r['w']:<5} {tag:10s} EN {r['EN']:6.3f}  "
                     f"ES {r['ES']:6.3f}  {r['layout']}")

    # knee candidates: best ES for bounded EN loss
    lines.append("")
    lines.append("--- candidates by EN-loss budget (vs cfgE EN 4.3877)")
    cands = sorted(results, key=lambda r: r["EN"])
    for budget in (0.1, 0.2, 0.3, 0.5):
        feasible = [r for r in results
                    if r["EN"] <= base["EN"] + budget]
        if feasible:
            best_es = min(feasible, key=lambda r: r["ES"])
            lines.append(
                f"  EN loss <= {budget:.1f}: w={best_es['w']}  "
                f"EN {best_es['EN']:.3f} (+{best_es['EN']-base['EN']:.3f})  "
                f"ES {best_es['ES']:.3f} ({best_es['ES']-base['ES']:+.3f})")

    txt = "\n".join(lines) + "\n"
    with open(OUT_TXT, "w") as f:
        f.write(txt)
    json.dump({"results": results, "refs": ref_pts},
              open(OUT_JSON, "w"), indent=1)
    print(txt)
    print(f"wrote {OUT_TXT}  ({time.time()-t_start:.0f}s total)")


if __name__ == "__main__":
    main()
