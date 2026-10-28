#!/usr/bin/env python3
"""human_geometry.py — the Gallium/Sturdy-class human tweaks.

Human post-release swaps target GEOMETRY the objective under-weights:
  - letters on hard lateral keys (top/bot-outer = stagger corners)
  - letters on a key with NO alt-fingering rescue
  - bad roll-direction on common bigrams (Sturdy sk/rl class)

This searches near-zero-cost single swaps that IMPROVE a geometry/feel
proxy, keeping only those the objective treats as free (dObj <= +0.003).
Report-only; nothing adopted.
"""
import sys

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer import Corpus
from search import Evaluator
from fingers import KEY_ORDER, KEYS
from compare import dead_of, slots_of, score_of

CFGE = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
BASE = 4.387726696836196
CORPUS = "/home/akai/git/layout-opencode-investigation/corpus/"
LETTERS = "abcdefghijklmnopqrstuvwxyz"

# geometry effort: lateral corner + no-alt + row (top ring/middle comfy per Engram)
def effort(i, k):
    key = KEYS[k]
    f = key.alts[0][0]
    base = {"I": 0.8, "M": 1.0, "R": 1.25, "P": 1.5}[f]
    row = {0: 1.0, 1: 0.9, 2: 1.15}[key.row]  # home lowest for index/middle
    col = i % 10
    lat = 1.0
    if col in (0, 9):
        lat = 1.35
    elif col in (1, 8):
        lat = 1.15
    alt_pen = {1: 0.4, 2: 0.15, 3: 0.0}.get(len(key.alts), 0.0)
    return base * row * lat * (1 + alt_pen)


def main():
    ce = Corpus.load(CORPUS)
    ev = Evaluator(ce, dead_of(CFGE))
    L = ce.L
    tot = sum(L.get(c, 0) for c in LETTERS) + sum(L.get(c, 0) for c in ",./;")
    # per-letter load, key index, effort
    li = {ch: CFGE.index(ch) for ch in LETTERS}
    load = {ch: L.get(ch, 0) / tot for ch in LETTERS}
    eff = {ch: effort(li[ch], KEY_ORDER[li[ch]]) for ch in LETTERS}
    anomaly = {ch: load[ch] * eff[ch] for ch in LETTERS}

    base_anom = sum(anomaly.values())
    print(f"baseline anomaly(load x effort) = {base_anom*100:.2f}")
    print()

    results = []
    for a in LETTERS:
        for b in LETTERS:
            if a >= b:
                continue
            i, j = li[a], li[b]
            lay = list(CFGE)
            lay[i], lay[j] = lay[j], lay[i]
            lay = "".join(lay)
            d_obj = score_of(ev, lay) - BASE
            if d_obj > 0.003:  # machine-free window
                continue
            # new anomaly
            ei = effort(li[a], KEY_ORDER[li[a]])  # a stays at i? no: a moves to j
            # recompute for the two moved letters only (others unchanged)
            new_anom_a = load[a] * effort(j, KEY_ORDER[j])   # a now on j's key
            new_anom_b = load[b] * effort(i, KEY_ORDER[i])   # b now on i's key
            d_anom = (new_anom_a + new_anom_b) - (anomaly[a] + anomaly[b])
            results.append((d_obj, d_anom, a + b, lay))
    results.sort()
    print(f"{'pair':4s} {'dObj':>7s} {'dAnom':>7s}  layout")
    print("-- improves geometry (dAnom<0) at ~0 objective cost --")
    for d_obj, d_anom, pair, lay in results:
        if d_anom < -0.0001:
            print(f"{pair:4s} {d_obj:+7.4f} {d_anom*100:+7.3f}  {lay}")
    print()
    print("-- free (dObj<=0.003) but geometry-neutral or worse --")
    for d_obj, d_anom, pair, lay in results:
        if d_anom >= -0.0001:
            print(f"{pair:4s} {d_obj:+7.4f} {d_anom*100:+7.3f}  {lay}")


if __name__ == "__main__":
    main()
