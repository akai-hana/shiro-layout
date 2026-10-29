"""Frequency-weighted fingering review: geometric model vs the project's current model.
usage: python3 -I review.py THETA.json VARIANT(A|B) LAYOUT30 NAME OUT.md"""
import os, sys, json, re
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import numpy as np
from utils import comfort as cg
from utils import gates as GT
import itertools
try:
    from . import oldmodel as om
except ImportError:
    import oldmodel as om

HERE = os.path.dirname(os.path.abspath(__file__))
PROJ = os.environ.get("SHIRO_FLOW_AUDIT") or os.path.join(HERE, "results", "archive", "flow_audit.txt")


def load_counts():
    bg, tg, mode = {}, {}, None
    for line in open(PROJ):
        if line.startswith("== ALL 551"): mode = "bg"; continue
        if line.startswith("== TOP 200 of"): mode = "tg"; continue
        if line.startswith("== TOP"): mode = None; continue
        m = re.match(r"\s+\d+ ([a-z]+)\s+(\d+)\s+([\d.]+)", line)
        if m and mode == "bg": bg[m.group(1)] = int(m.group(2))
        if m and mode == "tg": tg[m.group(1)] = int(m.group(2))
    return bg, tg


def pick_bi(tabs, k1, k2, n=3):
    a, b = cg.KI[k1], cg.KI[k2]
    pm = GT.penalty_mask((k1, k2))
    rows = [(float(tabs["T2"][a, fa, b, fb] + pm[fa, fb]), cg.FINGERS[fa] + cg.FINGERS[fb])
            for fa in range(4) for fb in range(4) if np.isfinite(tabs["T2"][a, fa, b, fb]) and not (a == b and fa != fb)]
    rows.sort(); return rows[:n]


def pick_tri(model, tabs, ks, n=2):
    t = cg.tri_block(model, tabs, *[cg.KI[k] for k in ks]) + GT.penalty_mask(tuple(ks))
    rows = sorted((float(t[x]), "".join(cg.FINGERS[f] for f in x)) for x in np.ndindex(t.shape) if np.isfinite(t[x]))
    return rows[:n]


def comps(model, tabs, k1, f1, k2, f2):
    d = cg.pair_breakdown(model, tabs, k1, f1, k2, f2)
    keep = {k: round(v, 2) for k, v in d.items() if k in ("vert", "cross", "spread", "roll", "leap", "key_a", "key_b") and abs(v) > 0.04}
    return keep


def main(theta_json, variant, layout, name, out):
    th = json.load(open(theta_json))
    GT.set_explicit(os.environ.get("SHIRO_GATES", "all"))
    model = cg.Model(theta=np.array([th[n] for n in cg.PNAMES]), disallow_full_MR=(variant == "A"))
    tabs = model.tables()
    pos = {ch: cg.KEYS[i] for i, ch in enumerate(layout) if ch.isalpha()}
    bg, tg = load_counts()
    tot_bg = sum(bg.values())
    rows, same_mass, agree_mass, alt_mass = [], 0, 0, 0
    for ng, n in sorted(bg.items(), key=lambda kv: -kv[1]):
        k1, k2 = pos[ng[0]], pos[ng[1]]
        if cg.H[cg.KI[k1]] != cg.H[cg.KI[k2]]:
            alt_mass += n; continue
        if k1 == k2: continue
        g = pick_bi(tabs, k1, k2, 3)
        o = om.best_bigram(k1, k2, 3)
        same_mass += n
        ag = g[0][1] == o[0][1]
        agree_mass += n if ag else 0
        marg = g[1][0] - g[0][0] if len(g) > 1 else 9.9
        rows.append(dict(ng=ng, n=n, keys=f"{k1}→{k2}", geo=g[0][1], gc=g[0][0], geo2=g[1][1] if len(g) > 1 else "", marg=marg,
                         old=o[0][1], oc=o[0][0], agree=ag, comp=comps(model, tabs, k1, g[0][1][0], k2, g[0][1][1]),
                         ocomp=None))
    L = []
    L.append(f"# Fingering review — {name}\n\nlayout `{layout}`  ·  model variant {variant} ({'full-span M/R & R/P reverse scissors impossible' if variant=='A' else 'full-span M/R priced, R/P impossible'})\n")
    L.append(f"- Cross-hand bigrams (no fingering decision): **{100*alt_mass/tot_bg:.1f}%** of bigram mass\n"
             f"- Same-hand bigrams: **{100*same_mass/tot_bg:.1f}%**; geometric model and current model pick the same fingering on **{100*agree_mass/same_mass:.1f}%** of that mass\n")
    def tbl(rs, extra=False):
        h = "| bigram | freq % | keys | geo pick | cost | runner-up (Δ) | current-model pick | cost | why (geo cost parts) |\n|---|---|---|---|---|---|---|---|---|\n"
        s = h
        for r in rs:
            s += (f"| {r['ng']} | {100*r['n']/tot_bg:.2f} | {r['keys']} | **{r['geo']}** | {r['gc']:.2f} | {r['geo2']} (+{r['marg']:.2f}) | "
                  f"{r['old']}{'' if r['agree'] else ' ✗'} | {r['oc']:.2f} | {r['comp']} |\n")
        return s
    L.append("## 1. Top 40 same-hand bigrams\n\n" + tbl(rows[:40]))
    dis = [r for r in rows if not r["agree"]]
    L.append(f"\n## 2. Disagreements with the current model, by frequency (top 40 of {len(dis)})\n\n" + tbl(dis[:40]))
    close = sorted([r for r in rows[:120] if r["marg"] < 0.20], key=lambda r: -r["n"])
    L.append(f"\n## 3. Closest calls among the 120 most common same-hand bigrams (runner-up within 0.20) — best questions to ask you\n\n" + tbl(close[:30]))
    # one-hand trigrams among top-200
    L.append("\n## 4. One-hand trigrams in the top 200\n\n| trigram | freq % | keys | geo pick | cost | runner-up | current-model pick | cost |\n|---|---|---|---|---|---|---|---|\n")
    tot_tg = sum(tg.values())
    for ng, n in sorted(tg.items(), key=lambda kv: -kv[1]):
        ks = [pos[c] for c in ng]
        if len({cg.H[cg.KI[k]] for k in ks}) != 1 or len(set(ks)) < 3: continue
        g = pick_tri(model, tabs, ks, 2)
        o = om.best_trigram(*ks, topn=1)
        L.append(f"| {ng} | {100*n/tot_tg:.2f} | {'→'.join(ks)} | **{g[0][1]}** | {g[0][0]:.2f} | {g[1][1]} (+{g[1][0]-g[0][0]:.2f}) | {o[0][1]}{'' if o[0][1]==g[0][1] else ' ✗'} | {o[0][0]:.2f} |\n")
    open(out, "w").write("".join(L))
    print("wrote", out, f"agree {100*agree_mass/same_mass:.1f}% of same-hand mass; disagreements {len(dis)}")


if __name__ == "__main__":
    main(*sys.argv[1:6])
