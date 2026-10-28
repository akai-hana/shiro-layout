#!/usr/bin/env python3
"""human_shortlist.py — decision table for human-observation swaps.

For the strongest feel-swap candidates, show the three views the community
actually triangulates on (Graphite/Enthium protocol: don't trust one number):

  1. our objective delta        (lower = machine-friendlier)
  2. standard-finger SFB delta  (keysolve view; Gallium J<->Z / X<->Q used this)
  3. pinky + top-lateral load delta (Kuntem / Sturdy feel)

Prints a ranked shortlist; nothing is adopted automatically.
"""
import sys

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
import numpy as np
from scorer import Corpus, get_tables
from search import Evaluator, N1, N2
from fingers import KEY_ORDER, KEYS
from compare import dead_of, slots_of, score_of

CFGE = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
BASE = 4.387726696836196
CORPUS = "/home/akai/git/layout-opencode-investigation/corpus/"
LETTERS = "abcdefghijklmnopqrstuvwxyz"

# candidate swaps: (pair, why)
CANDS = [
    ("qz", "Gallium J<->Z logic: rare<->rare, Q off hard bot-inner-index, Z off pinky"),
    ("jq", "J<->Q: J off top-inner-index (stretch), Q off bot-inner-index; both rare"),
    ("qx", "Q<->X: Q off hard bot-inner-index onto low-freq X (bot-middle ring)"),
    ("jx", "J<->X: J off top-inner-index onto low-freq X"),
    ("qj", "same as jq"),
    ("wv", "w off top-outer PINKY(P, no-alt) onto V; v off Y top-inner-index"),
    ("wy", "w off top-outer pinky(P) onto Y; y off G home-index"),
    ("wa", "a off top-outer-ring W off pinky-rescue zone; w onto A home-pinky"),
    ("aw", "a<->w: a onto W stays; this is the mirror — see note"),
    ("wu", "w off top-outer pinky(P) onto U; l/u block"),
    ("wq", "w off top-outer pinky(P) onto Q(free ring alt)"),
    ("bw", "b off bot-outer pinky(Z) onto W; w off top-outer pinky(P)"),
    ("bz", "b<->z: B(1.5%) off bot-pinky(Z) onto z's slot? z on B — mirror"),
    ("jw", "J<->W: j off top-inner-index onto P; w off top-outer pinky"),
]


def key_ids_of(lay):
    m = {ch: i for i, ch in enumerate(lay)}
    return m


def std_sfb(ce, lay):
    """standard-fingering SFB% (bigram denom, letters only, repeats incl)."""
    m = key_ids_of(lay)
    fdef = {}
    for ch in LETTERS:
        k = KEY_ORDER[m[ch]]
        fdef[ch] = KEYS[k].alts[0][0]
    tot = 0.0
    sfb = 0.0
    for bg, cnt in ce.B.items():
        if len(bg) != 2 or not all(c in LETTERS for c in bg):
            continue
        tot += cnt
        a, b = bg
        if fdef[a] == fdef[b] and m[a] != m[b]:
            sfb += cnt
    return 100 * sfb / tot


def pinky_lat_load(ce, lay):
    """pinky load% + top-lateral load% (letters+punct), for feel."""
    m = key_ids_of(lay)
    L = ce.L
    tot = sum(L.get(c, 0) for c in LETTERS) + \
        sum(L.get(c, 0) for c in ",./;")
    pinky = 0.0
    lat = 0.0
    for i, ch in enumerate(lay):
        k = KEY_ORDER[i]
        key = KEYS[k]
        load = L.get(ch, 0) / tot
        f = key.alts[0][0]
        if f == "P":
            pinky += load
        # top-lateral = top row outer columns (Q,P on top; Z,/ bottom-outer)
        if key.row == 0 and i % 10 in (0, 9):
            lat += load
        if key.row == 2 and i % 10 in (0, 9):
            lat += load
    return 100 * pinky, 100 * lat


def main():
    ce = Corpus.load(CORPUS)
    ev = Evaluator(ce, dead_of(CFGE))
    perm0 = list(slots_of(ev, CFGE))
    base_s = score_of(ev, CFGE)
    base_std = std_sfb(ce, CFGE)
    base_pinky, base_lat = pinky_lat_load(ce, CFGE)

    print(f"baseline: obj {base_s:.4f}  stdSFB {base_std:.3f}%  "
          f"pinky {base_pinky:.2f}%  top/bot-outer {base_lat:.3f}%")
    print()
    rows = []
    seen = set()
    for pair, why in CANDS:
        a, b = pair[0], pair[1]
        if a not in LETTERS or b not in LETTERS:
            continue
        if pair in seen:
            continue
        seen.add(pair)
        lay = list(CFGE)
        i, j = lay.index(a), lay.index(b)
        lay[i], lay[j] = lay[j], lay[i]
        lay = "".join(lay)
        s = score_of(ev, lay)
        std = std_sfb(ce, lay)
        pk, lt = pinky_lat_load(ce, lay)
        rows.append((s - base_s, pair, lay, s, std - base_std,
                     pk - base_pinky, lt - base_lat, why))
    rows.sort()
    print(f"{'pair':4s} {'dObj':>7s} {'dStdSFB':>8s} {'dPinky':>7s} "
          f"{'dLat':>7s}  layout / why")
    for d, pair, lay, s, dstd, dpk, dlt, why in rows:
        print(f"{pair:4s} {d:+7.4f} {dstd:+8.3f} {dpk:+7.2f} {dlt:+7.3f}  "
              f"{lay}")
        print(f"       {why}")
    print()
    print("read: dObj ~ +0.001..0.002 = machine-free (flat region).")
    print("      dStdSFB<0 = the swap ALSO wins under keysolve/standard view")
    print("      (Gallium J<->Z / X<->Q kept only such swaps).")
    print("      dPinky/dLat<0 = removes load from pinky / hard lateral keys.")


if __name__ == "__main__":
    main()
