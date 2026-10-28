#!/usr/bin/env python3
"""Test scorer2: old-vs-new cfgE score, ladder tiers, crossing, repeat charge."""
import sys, time
sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")

WIN = ";aiojvlnrwhfpeygtcmsbxquzkd,./"

import scorer, search
from compare import dead_of
from fingers import KEY_ORDER

def perm_of(lay, dead):
    slots = [k for k in KEY_ORDER if k not in dead.values()]
    slot_of = {k: i for i, k in enumerate(slots)}
    return [slot_of[KEY_ORDER[lay.index(ch)]] for ch in "abcdefghijklmnopqrstuvwxyz"]

# --- old model baseline
t0 = time.time()
scorer.TABLES = None; scorer.DEFAULT_TABLES = None
ce = scorer.Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")
ev_old = search.Evaluator(ce, dead_of(WIN))
s_old = ev_old.score(perm_of(WIN, dead_of(WIN)))
print(f"OLD model cfgE = {s_old:.4f}   (tables built in {time.time()-t0:.1f}s)")

# --- new model
import scorer2, search2
t0 = time.time()
scorer2.TABLES = None; scorer2.DEFAULT_TABLES = None
ev_new = search2.Evaluator(ce, dead_of(WIN))
s_new = ev_new.score(perm_of(WIN, dead_of(WIN)))
print(f"NEW model cfgE = {s_new:.4f}   (tables built in {time.time()-t0:.1f}s)")

# --- component checks
from scorer2 import (_bigram_cost, get_tables)
import ansi, comfort

BIG, TRI, BIG_F, TRI_F = get_tables()
FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}
idx = {ch: i for i, ch in enumerate(WIN)}

def show(bg):
    from fingers import KEYS as K2
    a, b = bg
    i, j = idx[a], idx[b]
    p = int(BIG_F[i, j])
    f1, f2 = FID2[p // 5], FID2[p % 5]
    k1, k2 = KEY_ORDER[i], KEY_ORDER[j]
    hand = K2[k1].hand
    cross = comfort.lateral_crossing(k1, f1, k2, f2, hand)
    print(f"  {bg} ({k1}->{k2}, {hand}): f={f1}{f2} BIG={BIG[i,j]:+.4f} "
          f"cross={cross:.3f} ansi_dist={ansi.dist(k1,k2):.3f}")

print("\nCrossing check (user's ED vs IK):")
show("ed")   # left M/I same col  -> aligned
show("ik")   # right hand same-col pair (stagger accentuates scissor)
print("\nKey positions in cfgE:")
for L in "nicde":
    pos = WIN.index(L)
    print(f"  letter '{L}' at KEY {KEY_ORDER[pos]} (row{pos//10} col{pos%10})")

# nc specifically
show("nc")

print("\nLadder check: M/I bigram costs by row-pair (via BIG table, letter pairs):")
# pick pairs realizing each ladder cell under cfgE
ladder_cases = {
    "(0,1) top-M,home-I": None, "(1,1) both home": None, "(0,0) both top": None,
    "(1,2) home-M,bot-I": None, "(2,2) both bot": None, "(0,2) top-M,bot-I": None,
    "(1,0) home-M,top-I": None, "(2,1) bot-M,home-I": None, "(2,0) bot-M,top-I": None,
}
# enumerate all M/I bigrams in table with their ladder cell
from comfort import mi_roll_penalty, _MI_LADDER
from fingers import KEYS as K

seen = {}
for i in range(30):
    for j in range(30):
        if i == j: continue
        p = int(BIG_F[i, j]); f1, f2 = FID2[p // 5], FID2[p % 5]
        k1, k2 = KEY_ORDER[i], KEY_ORDER[j]
        if K[k1].hand != K[k2].hand: continue
        if {f1, f2} != {"M", "I"}: continue
        v = mi_roll_penalty(k1, f1, k2, f2, K[k1].hand)
        if v is None: continue
        mrow = int(round(ansi.cy(k1 if f1 == "M" else k2) - 0.5))
        irow = int(round(ansi.cy(k2 if f1 == "M" else k1) - 0.5))
        seen.setdefault((mrow, irow), []).append((k1 + k2, f1 + f2, BIG[i, j]))
for cell, lad in sorted(_MI_LADDER.items()):
    items = seen.get(cell, [])
    if items:
        items.sort(key=lambda x: x[2])
        ex = items[0]
        print(f"  cell{cell} ladder={lad:.2f}  e.g. {ex[0]} f={ex[1]} BIG={ex[2]:+.3f}")
    else:
        print(f"  cell{cell} ladder={lad:.2f}  (no bigram in cfgE realizes this)")

print("\nRepeat charge check:")
i = idx["l"]
p = int(BIG_F[i, i]); f1, f2 = FID2[p // 5], FID2[p % 5]
print(f"  'll': f={f1}{f2}  _bigram_cost(l,f1,l,f2) = {_bigram_cost('l',f1,'l',f2):.3f} "
      f"(expect W_REPEAT=0.6 or +0.5 swap)")
print(f"  W_REPEAT={comfort.W_REPEAT} W_CROSS={comfort.W_CROSS}")
