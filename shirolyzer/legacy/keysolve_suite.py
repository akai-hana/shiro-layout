#!/usr/bin/env python3
"""A2+A3: keysolve-compatible metric suite + quartile-band placement.

Python port of keysolve-web js/{classify.mjs,stats.mjs} over its baked corpora,
so SFB/LSB/HSB/FSB/SFS/LSS/HSS/FSS/ALT/ROL/ONE/RED land on exactly the
definitions behind percentiles.json (community quartile boundaries).

Board: ortho (no stagger X shifts), standard finger map, right thumb,
finger condition 'None' (patternQuality always true) — keysolve defaults.
Denominator: all gram counts (unmapped chars included), as in stats.mjs.
"""
import json
import sys

ROOT = "/tmp/opencode/analyzers/keysolve-web"
OUT_TXT = "/home/akai/git/layout-opencode-investigation/results/keysolve_report.txt"
OUT_JSON = "/home/akai/git/layout-opencode-investigation/results/keysolve_report.json"

FINGER_MAP = ([0, 1, 2, 3, 3, 6, 6, 7, 8, 9]) * 3   # STANDARD, right thumb=5
THUMB = 5
FINGERS = ["LP", "LR", "LM", "LI", "LT", "RT", "RI", "RM", "RR", "RP"]
LOWER_BETTER = {"SFB", "LSB", "HSB", "FSB", "SFS", "LSS", "HSS", "FSS", "RED"}
HIGHER_BETTER = {"ALT", "ROL"}
CORE = ["SFB", "LSB", "HSB", "FSB", "SFS", "LSS", "HSS", "FSS",
        "ALT", "ROL", "ONE", "RED"]

LAYOUTS = {
    "cfgE_semiQ (ours)":  ";aiojvlnrwhfpeygtcmsbxquzkd,./",
    "cfgG_semiP (alt)":   "faiojwlrn;hgpeyvtcmsbqxuzkd,./",
    "Sturdy":             "vmlcpxfoujsrtdy.naeizkqgwbh';,",
    "Graphite":           "bldwz'foujnrtsgyhaeiqxmcvkp.-/",
    "Colemak-DH":         "qwfpbjluy;arstgmneiozxcdvkh,./",
    "Canary":             "wlypkzxou;crstbfneiajvdgqmh/,.",
    "Semimak":            "flhvzqwuoysrntkcdeaix'bmjpg/,/",
    "Gallium":            "bldcvjfou,nrtsgyhaeixqmwzkp';.",
    "Colemak":            "qwfpgjluy;arstdhneiozxcvbkm,./",
    "QWERTY":             "qwertyuiopasdfghjkl;zxcvbnm,./",
}


def finger(idx):
    return FINGER_MAP[idx] if idx is not None and idx < 30 else THUMB


def column(idx):
    return idx % 10


def row(idx):
    return idx // 10


def xshift(idx):  # board === 'ortho' → no stagger shift
    return column(idx)


def hand(idx):
    return 0 if finger(idx) < 5 else 1


def ordered(idxs):
    inc = dec = True
    for a, b in zip(idxs, idxs[1:]):
        if finger(b) < finger(a):
            inc = False
        if finger(b) > finger(a):
            dec = False
    return inc or dec


def bigrams(k):
    out = []
    if finger(k[0]) == finger(k[1]) and k[0] != k[1]:
        out.append("SF")
        return out
    if (hand(k[0]) == hand(k[1])
            and abs(finger(k[0]) - finger(k[1])) == 1
            and abs(xshift(k[0]) - xshift(k[1])) >= 2
            and finger(k[0]) not in (4, 5) and finger(k[1]) not in (4, 5)):
        out.append("LS")
    if ((row(k[0]) - row(k[1]) == -1 and hand(k[0]) == hand(k[1])
         and finger(k[1]) in (1, 2, 7, 8))
            or (row(k[0]) - row(k[1]) == 1 and hand(k[0]) == hand(k[1])
                and finger(k[0]) in (1, 2, 7, 8))):
        out.append("HS")
    if ((row(k[0]) - row(k[1]) == -2 and hand(k[0]) == hand(k[1])
         and finger(k[1]) in (1, 2, 7, 8))
            or (row(k[0]) - row(k[1]) == 2 and hand(k[0]) == hand(k[1])
                and finger(k[0]) in (1, 2, 7, 8))):
        out.append("FS")
    return out


def trigrams(k):
    out = []
    if hand(k[0]) == hand(k[2]) and hand(k[0]) != hand(k[1]):
        out.append("ALT")
    if (len({hand(x) for x in k}) == 2 and len({finger(x) for x in k}) == 3
            and hand(k[0]) != hand(k[2])):
        out.append("ROL")
    if (len({hand(x) for x in k}) == 1 and len({finger(x) for x in k}) == 3
            and ordered(k)):
        out.append("ONE")
    if (len({hand(x) for x in k}) == 1 and finger(k[0]) != finger(k[1])
            and finger(k[1]) != finger(k[2]) and not ordered(k)):
        out.append("RED")
    return out


def quadgrams(k):
    out = []
    hs = [hand(x) for x in k]
    fs = [finger(x) for x in k]
    if hs[0] == hs[2] and hs[1] == hs[3] and hs[0] != hs[1]:
        if len(set(fs)) == 4:
            out += ["CA", "SA"]
        else:
            out.append("SA")
    if len(set(hs)) == 2 and len(set(fs)) == 4 and hs[0] == hs[1] != hs[2] == hs[3]:
        out.append("CR")
    if len(set(hs)) == 2 and hs[0] != hs[1] == hs[2] != hs[3] and fs[1] != fs[2]:
        out.append("TR")
        if len(set(fs)) == 4:
            out.append("BT")
    if len(set(hs)) == 1 and len(set(fs)) == 4 and ordered(k):
        out.append("4R")
    if (len(set(hs)) == 1 and fs[0] != fs[1] and fs[1] != fs[2]
            and fs[2] != fs[3] and not ordered(k)):
        out.append("RD")
    return out


def classify(keys):
    n = len(keys)
    if n == 2:
        return bigrams(keys)
    if n == 3:
        return trigrams(keys)
    if n == 4:
        return quadgrams(keys)
    return []


def analyze(layout_str):
    layout = {ch: i for i, ch in enumerate(layout_str)}  # keysolve: last wins
    res = {}
    for suffix, fname in [("B", "bigrams"), ("S", "skipgrams"),
                          ("Q", "tetragrams"), ("", "trigrams")]:
        grams = json.load(open(f"{ROOT}/corpora/{fname}.json"))
        curr, total = {}, 0
        for gram, count in grams.items():
            total += count
            idxs = [layout.get(c) for c in gram.lower()]
            if any(i is None for i in idxs):
                continue
            for stat in classify(idxs):
                curr[stat + suffix] = curr.get(stat + suffix, 0) + count
        for stat, count in curr.items():
            res[stat] = count / total

    monograms = json.load(open(f"{ROOT}/corpora/monograms.json"))
    curr, total = {}, 0
    for gram, count in monograms.items():
        idx = layout.get(gram)
        f = FINGERS[FINGER_MAP[idx]] if idx is not None and idx < 30 else None
        if f is None:
            continue
        curr[f] = curr.get(f, 0) + count
        total += count
    for f, count in curr.items():
        res[f] = count / total
    res["LH"] = sum(res.get(x, 0) for x in ["LI", "LM", "LR", "LP", "LT"])
    res["RH"] = sum(res.get(x, 0) for x in ["RI", "RM", "RR", "RP", "RT"])
    return res


def band(value, bounds, lower_better):
    """5 bands split by 4 ascending boundaries; band 1 = best side."""
    if lower_better:
        return sum(1 for b in bounds if value >= b) + 1
    return sum(1 for b in bounds if value <= b) + 1


def main():
    pct = json.load(open(f"{ROOT}/percentiles.json"))
    results = {}
    for name, lay in LAYOUTS.items():
        results[name] = analyze(lay)

    lines = ["A2+A3 — keysolve-compatible metrics (its corpora + classify defs)",
             "boundaries: quartiles.json 4 ascending values/metric; band 1 = best",
             ""]
    # core metric table
    for metric in CORE:
        bounds = [b * 100 for b in pct[metric]]
        lb = metric in LOWER_BETTER
        hb = metric in HIGHER_BETTER
        dirn = "lower-better" if lb else ("higher-better" if hb else "neutral")
        lines.append(f"--- {metric} %  ({dirn})  bounds " +
                     "/".join(f"{b:.2f}" for b in bounds))
        order = sorted(results,
                       key=lambda n: results[n][metric],
                       reverse=(metric in HIGHER_BETTER))
        for n in order:
            v = results[n][metric] * 100
            bnd = band(results[n][metric], pct[metric], not hb)
            lines.append(f"  {n:22s} {v:7.2f}  band {bnd}")
        lines.append("")

    # finger shares vs quartiles
    lines.append("--- finger shares % (keysolve quartiles)")
    for f in ["LP", "LR", "LM", "LI", "RI", "RM", "RR", "RP", "LH", "RH"]:
        bounds = [b * 100 for b in pct[f]]
        rowp = " ".join(f"{results[n][f]*100:.1f}" for n in LAYOUTS)
        lines.append(f"  {f:3s} ours {results['cfgE_semiQ (ours)'][f]*100:5.1f}  "
                     f"bounds {bounds[0]:.1f}/{bounds[-1]:.1f}  all: {rowp}")

    txt = "\n".join(lines) + "\n"
    print(txt)
    with open(OUT_TXT, "w") as f:
        f.write(txt)
    json.dump(results, open(OUT_JSON, "w"), indent=1)
    print(f"wrote {OUT_TXT}, {OUT_JSON}")


if __name__ == "__main__":
    main()
