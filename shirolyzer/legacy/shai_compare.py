#!/usr/bin/env python3
"""A1: score winner + community refs on the Shai iWeb corpus (community-standard
stats source), apples-to-apples with the Norvig-based final_comparison.

Corpus: corpus/shai/ built from genkey's shai-iweb.json (letters-only a-z pairs
and triples, mirroring the Norvig convention: no space, no punctuation).
"""
import json
import sys

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer import Corpus
from search import Evaluator
from compare import dead_of, score_of, REF

NORVIG = "/home/akai/git/layout-opencode-investigation/corpus/"
SHAI = "/home/akai/git/layout-opencode-investigation/corpus/shai/"
OUT_JSON = "/home/akai/git/layout-opencode-investigation/results/shai_comparison.json"
OUT_TXT = "/home/akai/git/layout-opencode-investigation/results/shai_comparison.txt"

GRAPHITE = "bldwz'fouj" "nrtsgyhaei" "qxmcvkp.-/"

CONFIGS = {
    "cfgE_semiQ (winner)": ";aiojvlnrwhfpeygtcmsbxquzkd,./",
    "cfgG_semiP (alt)": "faiojwlrn;hgpeyvtcmsbqxuzkd,./",
    "cfgD_slashQ": "/aiojvlnrwsfpeygtcm;xbquzkd,.h",
    "cfgF_aposQ": "'aiojvlnrwsfpeygtcm;xbquzkd,.h",
}

LAYOUTS = {}
LAYOUTS.update(CONFIGS)
LAYOUTS.update({
    "Sturdy": REF["Sturdy"],
    "Graphite": GRAPHITE,
    "Colemak-DH": REF["Colemak-DH"],
    "Colemak": REF["Colemak"],
    "Semimak": REF["Semimak"],
    "Gallium": REF["Gallium"],
    "Canary": REF["Canary"],
    "QWERTY": REF["QWERTY"],
})


def score_all(corpus_dir):
    corpus = Corpus.load(corpus_dir)
    out = {}
    for name, lay in LAYOUTS.items():
        ev = Evaluator(corpus, dead_of(lay))
        out[name] = score_of(ev, lay)
    return out


def main():
    norvig = score_all(NORVIG)
    shai = score_all(SHAI)

    # cross-check Norvig column against archived final_comparison.json
    fc = json.load(open(NORVIG.replace("corpus/", "results/") + "final_comparison.json"))
    expected = {"cfgE_semiQ (winner)": 4.3877, "cfgG_semiP (alt)": 4.401,
                "Sturdy": 5.5756, "Graphite": 5.628, "Colemak-DH": 5.670,
                "Colemak": 5.6822, "Semimak": 5.6999, "Gallium": 5.6597,
                "Canary": 5.9287, "QWERTY": 8.7119}
    for k, exp in expected.items():
        got = norvig[k]
        flag = "OK" if abs(got - exp) < 2e-3 else f"MISMATCH (exp {exp})"
        if flag != "OK":
            print(f"cross-check {k}: {got:.4f} {flag}")

    rows = sorted(LAYOUTS, key=lambda n: shai[n])
    norvig_rank = {n: i + 1 for i, n in enumerate(sorted(LAYOUTS, key=lambda n: norvig[n]))}

    lines = []
    lines.append("A1 — Shai iWeb corpus vs Norvig (our objective, letters-only a-z)")
    lines.append(f"{'layout':22s} {'shai':>8s} {'rank':>5s} {'norvig':>8s} {'nrank':>5s}  layout-str")
    for i, n in enumerate(rows, 1):
        lines.append(f"{n:22s} {shai[n]:8.4f} {i:5d} {norvig[n]:8.4f} "
                     f"{norvig_rank[n]:5d}  {LAYOUTS[n]}")
    top4 = rows[:4]
    lines.append("")
    lines.append(f"shai top4: {', '.join(top4)}")
    lines.append(f"norvig top4: {', '.join(sorted(LAYOUTS, key=lambda n: norvig[n])[:4])}")
    # spearman-ish: exact rank match count
    same = sum(1 for n in LAYOUTS if norvig_rank[n] == rows.index(n) + 1)
    lines.append(f"exact rank matches: {same}/{len(LAYOUTS)}")

    txt = "\n".join(lines) + "\n"
    print(txt)
    with open(OUT_TXT, "w") as f:
        f.write(txt)
    json.dump({"shai": shai, "norvig": norvig,
               "shai_rank": {n: rows.index(n) + 1 for n in rows},
               "norvig_rank": norvig_rank, "layouts": LAYOUTS},
              open(OUT_JSON, "w"), indent=1)
    print(f"wrote {OUT_TXT} and {OUT_JSON}")


if __name__ == "__main__":
    main()
