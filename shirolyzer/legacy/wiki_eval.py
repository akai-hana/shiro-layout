#!/usr/bin/env python3
"""D5: extract prose from wiki_api.json, build corpus/wiki, score field.

Wikitext strip: templates, tables, refs, file/category links, headings,
external links — good-enough for letter/bigram/trigram frequencies.
"""
import json
import os
import re
import sys

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer import Corpus
from search import Evaluator
from compare import dead_of, score_of

WIKI = "/tmp/opencode/wiki_api.json"
OUTDIR = "/home/akai/git/layout-opencode-investigation/corpus/wiki/"
LETTERS = set("abcdefghijklmnopqrstuvwxyz")


def strip_wikitext(text):
    text = re.sub(r"<ref[^>]*>.*?</ref>", " ", text, flags=re.S)
    text = re.sub(r"<ref[^>]*/>", " ", text)
    text = re.sub(r"\{\{[^{}]*\}\}", " ", text)       # templates (one level)
    text = re.sub(r"\{\{[^{}]*\}\}", " ", text)
    text = re.sub(r"\[\[Category:[^\]]*\]\]", " ", text)
    text = re.sub(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]", r"\1", text)
    text = re.sub(r"\[https?://[^\]]*\]", " ", text)
    text = re.sub(r'^\s*\|.*$', " ", text, flags=re.M)  # table rows
    text = re.sub(r"[|{}<> =]", " ", text)
    text = re.sub(r"'{2,}", " ", text)
    return text


def main():
    L, B, T = {}, {}, {}
    n_articles = 0
    for line in open(WIKI, "rb"):
        if not line.strip():
            continue
        d = json.loads(line)
        pages = d.get("query", {}).get("pages", {})
        for p in pages.values():
            revs = p.get("revisions")
            if not revs:
                continue
            text = revs[0].get("slots", {}).get("main", {}).get("*", "")
            if not text:
                continue
            n_articles += 1
            prose = strip_wikitext(text).lower()
            run = []
            for ch in prose:
                if ch in LETTERS:
                    run.append(ch)
                else:
                    if len(run) >= 1:
                        for c in run:
                            L[c] = L.get(c, 0) + 1
                    for i in range(len(run) - 1):
                        k = "".join(run[i:i+2]); B[k] = B.get(k, 0) + 1
                    for i in range(len(run) - 2):
                        k = "".join(run[i:i+3]); T[k] = T.get(k, 0) + 1
                    run = []
            if run:
                for c in run:
                    L[c] = L.get(c, 0) + 1
                for i in range(len(run) - 1):
                    k = "".join(run[i:i+2]); B[k] = B.get(k, 0) + 1
                for i in range(len(run) - 2):
                    k = "".join(run[i:i+3]); T[k] = T.get(k, 0) + 1

    os.makedirs(OUTDIR, exist_ok=True)
    for name, dct in (("letter", L), ("bigram", B), ("trigram", T)):
        json.dump(dct, open(OUTDIR + name + "_freq.json", "w"))
    print(f"articles {n_articles}  L {sum(L.values())} chars, "
          f"B {len(B)}, T {len(T)}")

    ce = Corpus.load(OUTDIR)
    cw = Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")

    layouts = {
        "cfgE":     ";aiojvlnrwhfpeygtcmsbxquzkd,./",
        "cfgG":     "faiojwlrn;hgpeyvtcmsbqxuzkd,./",
        "cfgE-punct": ".aiozvlnrbhgpeymtcwsfjxu/kd,;q",
        "hybrid-35": ";aemjwcioksfgnhptubrqzylxvd,./",
        "hybrid-80": ";eiowgcnrvszpafmtbhljkyuxqd,./",
        "Sturdy":   "vmlcpxfoujsrtdy.naeizkqgwbh';,",
        "Graphite": "bldwz'foujnrtsgyhaeiqxmcvkp.-/",
        "Gallium":  "bldcvjfou,nrtsgyhaeixqmwzkp';.",
        "Colemak-DH": "qwfpbjluy;arstgmneiozxcdvkh,./",
        "QWERTY":   "qwertyuiopasdfghjkl;zxcvbnm,./",
    }
    for key, lay in list(layouts.items()):
        if len(lay) != 30 or len(set(lay)) != 30:
            print(f"  (skipping bad layout {key}: {lay!r})")
            del layouts[key]

    print(f"\n{'layout':12s} {'wiki':>8s} {'iweb':>8s}")
    rows = []
    for name, lay in layouts.items():
        # per-layout dead set — slots_of requires evaluator.match layout
        s_w = score_of(Evaluator(ce, dead_of(lay)), lay)
        s_e = score_of(Evaluator(cw, dead_of(lay)), lay)
        rows.append((s_w, name, lay, s_e))
    rows.sort()
    for s_w, name, lay, s_e in rows:
        print(f"{name:12s} {s_w:8.4f} {s_e:8.4f}  {lay}")

    out = ["D5 — Wikipedia (200 featured articles) third-source check",
           f"articles {n_articles}  {sum(L.values())} letters  "
           f"{len(T)} trigram types", "",
           f"{'layout':12s} {'wiki':>8s} {'iweb':>8s}  layout"]
    for s_w, name, lay, s_e in rows:
        out.append(f"{name:12s} {s_w:8.4f} {s_e:8.4f}  {lay}")
    open("/home/akai/git/layout-opencode-investigation/results/d5_wiki.txt",
         "w").write("\n".join(out) + "\n")


if __name__ == "__main__":
    main()
