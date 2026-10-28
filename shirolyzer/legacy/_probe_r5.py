"""Round-5 probe: O->L MR viable, fxw=IMR, wxf=MII, X->Q=IM, crossings."""
import sys
sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")

from fingers import KEYS
import scorer2
from scorer2 import FID, _bigram_cost, _trigram_cost

INV = {v: k for k, v in FID.items()}


def eff(n, f):
    from comfort import travel_cost, TRAVEL_SCALE
    return travel_cost(n, f, KEYS[n].hand) + TRAVEL_SCALE * dict(KEYS[n].alts)[f]


def opts(n):
    return [f for f, _ in KEYS[n].alts]


def bigram(k1, k2, targets=()):
    rows = []
    for f1 in opts(k1):
        for f2 in opts(k2):
            c = eff(k1, f1) + eff(k2, f2) + _bigram_cost(k1, f1, k2, f2)
            rows.append((c, f1, f2))
    rows.sort()
    print(f"{k1}->{k2}:")
    for c, f1, f2 in rows[:4]:
        mark = ""
        if (f1, f2) in targets:
            mark = "  <-- target"
        print(f"    {c:7.3f}  {f1}{f2}{mark}")
    for (a, b) in targets:
        hit = [r for r in rows if (r[1], r[2]) == (a, b)]
        if hit and hit[0] not in rows[:4]:
            print(f"    {hit[0][0]:7.3f}  {a}{b}  <-- target (not top4)")
        elif not hit:
            print(f"    (target {a}{b} not available)")


def trigram(k1, k2, k3, topn=6):
    rows = []
    for f1 in opts(k1):
        for f2 in opts(k2):
            for f3 in opts(k3):
                c = (eff(k1, f1) + eff(k2, f2) + eff(k3, f3) +
                     _bigram_cost(k1, f1, k2, f2) +
                     _bigram_cost(k2, f2, k3, f3) +
                     _trigram_cost(k1, f1, k2, f2, k3, f3))
                rows.append((c, f1, f2, f3))
    rows.sort()
    print(f"{k1}{k2}{k3}:")
    for c, f1, f2, f3 in rows[:topn]:
        print(f"    {c:7.3f}  {f1}{f2}{f3}")


if __name__ == "__main__":
    scorer2.TABLES = None
    bigram("O", "L", targets=[("M", "R"), ("R", "R"), ("R", "M")])
    bigram("L", "O", targets=[("R", "M"), ("M", "M")])
    bigram("X", "Q", targets=[("I", "M"), ("R", "P"), ("R", "M")])
    bigram("Q", "X", targets=[("M", "I"), ("P", "R")])
    bigram("S", "W", targets=[("I", "M"), ("R", "M"), ("M", "R")])
    bigram("W", "S", targets=[("M", "I"), ("M", "R")])
    bigram("X", "S", targets=[("I", "M"), ("M", "R"), ("R", "M")])
    bigram("F", "X", targets=[("I", "M"), ("M", "I")])
    bigram("X", "F", targets=[("M", "I"), ("I", "M")])
    print()
    trigram("F", "X", "W")
    trigram("W", "X", "F")
    trigram("O", "R", "M")
    trigram("X", "Q", "Z")
