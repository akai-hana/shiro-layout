"""
validate.py — Sanity checks for the fingering/scoring model.

1. Scissor detector vs canonical QWERTY scissors (CR, DR, KO, PL) and the
   arched-home rescues (W-D, K-O should be heavily discounted by rest).
2. Reference layouts scored under DEFAULT fingering — ordering should roughly
   match community consensus (QWERTY worst; Gallium/Canary/Sturdy/Colemak-DH
   much better).
3. Alt-fingering benefit: score each layout with alts vs default tables.
"""
import sys
sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from fingers import scissor_cost, KEYS, KEY_ORDER
from scorer import Corpus, get_tables, get_default_tables
from metrics import print_grid

REF = {
    "QWERTY":     "qwertyuiop" "asdfghjkl;" "zxcvbnm,./",
    "Colemak":    "qwfpgjluy;" "arstdhneio" "zxcvbkm,./",
    "Colemak-DH": "qwfpbjluy;" "arstgmneio" "zxcdvkh,./",
    "Canary":     "wlypkzxou;" "crstbfneia" "jvdgqmh/,.",
    "Sturdy":     "vmlcpxfouj" "srtdy.naei" "zkqgwbh';,",
    "Gallium":    "bldcvjfou," "nrtsgyhaei" "xqmwzkp';.",
    "Semimak":    "flhvzqwuoy" "srntkcdeai" "x'bmjpg/,/",
}

# fix Canary/Sturdy/Semimak length (should be exactly 30)
for k in list(REF):
    if len(REF[k]) != 30:
        print(f"WARN {k} len={len(REF[k])}")


def scissors_table():
    # Test with explicit QWERTY fingering (the canonical scissor examples),
    # then with this model's default fingering.
    tests = [
        # (k1,f1, k2,f2, expect_nonzero, note)
        ("C","M", "R","I", True,  "QWERTY CR (FSB, IM wiki-type)"),
        ("D","M", "R","I", True,  "QWERTY DR (HSB, IM wiki-type)"),
        ("K","M", "O","R", False, "QWERTY KO is rescued here: O is ring REST in arched model"),
        ("P","P", "L","R", True,  "QWERTY PL (RP wiki-type)"),
        ("W","R", "D","M", False, "arched rescue: W is ring rest (model default)"),
        ("O","R", "K","M", False, "arched rescue: O is ring rest (model default)"),
        ("E","M", "S","R", False, "reverse MR (middle up, ring down): free"),
        ("R","I", "F","I", False, "same finger (not a scissor)"),
        ("T","M", "F","I", False, "reverse IM (middle over index): tolerated"),
        ("D","M", "T","I", True,  "retracted-middle/extended-index FSB: intolerant"),
    ]
    print("== scissor model checks ==")
    for n1,f1,n2,f2, expect, note in tests:
        cost = scissor_cost(n1, f1, n2, f2)
        ok = (cost > 0.3) == expect
        print(f"  {'OK ' if ok else 'XX '} {n1}({f1})-{n2}({f2}) = {cost:5.2f}  {note}")
    # with alts: Q-S via S->middle is an MR scissor -> should be expensive
    cost = scissor_cost("Q", "R", "S", "M")
    print(f"  Q(ring)-S(middle-alt) scissor = {cost:.2f} (expect MR x rest mult ~2.5)")
    cost = scissor_cost("Q", "R", "S", "I")
    print(f"  Q(ring)-S(index-alt) scissor = {cost:.2f} (expect 0, non-adjacent)")
    # model-default checks
    cost = scissor_cost("R", "M", "D", "M")  # same finger actually
    print(f"  R(m)-D(m) same-finger check = {cost:.2f} (0: same finger handled as SFB)")


def ref_scores(prefix):
    print(f"\n== reference layouts, corpus={prefix} ==")
    corpus = Corpus.load(prefix)
    BIG, TRI, _, _ = get_tables()
    BIGd, TRId = get_default_tables()
    for name, layout in REF.items():
        # score with default tables (no alts) and with alt tables
        s_def = _score_with(layout, corpus, BIGd, TRId)
        s_alt = _score_with(layout, corpus, BIG, TRI)
        print(f"  {name:11s} default={s_def:8.4f}   alts={s_alt:8.4f}   "
              f"gain={s_def - s_alt:7.4f}")


def _score_with(layout, corpus, BIG, TRI):
    m = {ch: i for i, ch in enumerate(layout)}
    Nloc = 31
    cost = 0.0
    n = 0.0
    for bg, cnt in corpus.B.items():
        if len(bg) == 2 and bg[0] in m and bg[1] in m:
            cost += BIG[m[bg[0]], m[bg[1]]] * cnt
            n += cnt
    for tg, cnt in corpus.T.items():
        if len(tg) == 3 and all(c in m for c in tg):
            cost += TRI[m[tg[0]], m[tg[1]], m[tg[2]]] * cnt
            n += cnt
    return cost / n


if __name__ == "__main__":
    scissors_table()
    prefix = sys.argv[1] if len(sys.argv) > 1 else "/tmp/opencode/corpus/"
    ref_scores(prefix)
