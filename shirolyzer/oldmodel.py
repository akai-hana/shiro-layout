"""Thin wrapper around the user's existing (round-7) model for baseline picks.
Reads the project read-only; never writes into it."""
import sys, os
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
HERE = os.path.dirname(os.path.abspath(__file__))
PROJ = os.environ.get("SHIRO_ANALYSIS_DIR") or os.path.join(HERE, "legacy", "round8")
sys.path.append(os.path.dirname(PROJ))
sys.path.append(PROJ)
import ansi, comfort
from fingers import KEYS, KEY_ORDER
from comfort import travel_cost, TRAVEL_SCALE
from scorer2 import _bigram_cost, _trigram_cost, _alt_context_penalty

def opts(n):
    return [f for f, _ in KEYS[n].alts]

def eff(n, f):
    alts = dict(KEYS[n].alts)
    if f not in alts:
        return 1e6        # finger not licensed for this key in the shipped alt table: unreachable
    return travel_cost(n, f, KEYS[n].hand) + TRAVEL_SCALE * alts[f]

def best_bigram(k1, k2, topn=3):
    rows = []
    for f1 in opts(k1):
        for f2 in opts(k2):
            if k1 == k2 and f1 != f2: continue
            c = (eff(k1, f1) + eff(k2, f2) + _alt_context_penalty(k1, f1, (k2,)) +
                 _alt_context_penalty(k2, f2, (k1,)) + _bigram_cost(k1, f1, k2, f2))
            rows.append((c, f1 + f2))
    rows.sort()
    return rows[:topn]

def best_trigram(k1, k2, k3, topn=3):
    rows = []
    for f1 in opts(k1):
        for f2 in opts(k2):
            for f3 in opts(k3):
                if (k1 == k2 and f1 != f2) or (k2 == k3 and f2 != f3) or (k1 == k3 and f1 != f3):
                    continue
                c = (eff(k1, f1) + eff(k2, f2) + eff(k3, f3) +
                     _alt_context_penalty(k1, f1, (k2, k3)) +
                     _alt_context_penalty(k2, f2, (k1, k3)) +
                     _alt_context_penalty(k3, f3, (k1, k2)) +
                     _bigram_cost(k1, f1, k2, f2) + _bigram_cost(k2, f2, k3, f3) +
                     _trigram_cost(k1, f1, k2, f2, k3, f3))
                rows.append((c, f1 + f2 + f3))
    rows.sort()
    return rows[:topn]

if __name__ == "__main__":
    for a, b in [("O","L"),("X","W"),("U","M"),("S","W"),("A","W")]:
        print(a, b, best_bigram(a, b))
    for t in [("F","X","W"),("W","X","F")]:
        print(t, best_trigram(*t))
