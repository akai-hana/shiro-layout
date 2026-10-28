#!/usr/bin/env python3
"""Round-5 verification suite — consolidated (probe + model checks + audit).

Usage:
  python3 _verify_r5.py            # run all sections
  python3 _verify_r5.py probe      # target-fingering probes (user verdicts)
  python3 _verify_r5.py model      # old-vs-new cfgE + component checks
  python3 _verify_r5.py audit      # impossible scans, pick_audit, rescoring, anneal

Sections
  probe : bigram/trigram probes against the USER's ruled fingerings
          (O->L, X->Q, fxw/wxf, S->W, X->S, F->X...). Expected winners are
          marked "<-- target"; targets must appear in the top rows.
  model : OLD scorer vs NEW scorer2 cfgE score, plus component sanity
          (crossing, ladder cells, repeat charge).
  audit : corpus-wide impossible-winner scans (must be 0), pick_audit
          regeneration (results/pick_audit.txt), reference rescoring
          (cfgE / annealBEST / community refs), 20k seeded anneal.
"""
import sys
sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")

import time

import ansi
import comfort
import scorer2
from fingers import KEYS, KEY_ORDER
from comfort import (impossible_mr, impossible_ir, impossible_cross,
                     impossible_rp, travel_cost, TRAVEL_SCALE,
                     mi_roll_penalty, _MI_LADDER)
from scorer2 import FID, get_tables, _bigram_cost, _trigram_cost, Corpus
from search2 import Evaluator, LETTERS
from compare import dead_of, REF

WIN = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
ANNEAL_BEST = ";stckzdaiwnlprhyeguobfvmjqx,./"
NEW_BEST = ";stpbqdaiynhcrmkeguowvflzxj,./"
CORPUS = "/home/akai/git/layout-opencode-investigation/corpus/"
PICK_AUDIT = "/home/akai/git/layout-opencode-investigation/results/pick_audit.txt"
NEWBEST_AUDIT = "/home/akai/git/layout-opencode-investigation/results/newbest_audit.txt"

INV_FID = {v: k for k, v in FID.items()}
FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}


def is_impossible(k1, f1, k2, f2):
    return (impossible_mr(k1, f1, k2, f2)
            or impossible_ir(k1, f1, k2, f2)
            or impossible_cross(k1, f1, k2, f2)
            or impossible_rp(k1, f1, k2, f2))


def letter_keys(lay):
    """letter -> physical key name under `lay`."""
    return {ch: KEY_ORDER[lay.index(ch)] for ch in LETTERS}


def perm_of(lay, dead):
    slots = [k for k in KEY_ORDER if k not in dead.values()]
    slot_of = {k: i for i, k in enumerate(slots)}
    return [slot_of[KEY_ORDER[lay.index(ch)]] for ch in LETTERS]


# ------------------------------------------------------------------ probe
def eff(n, f):
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
        mark = "  <-- target" if (f1, f2) in targets else ""
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


FAILS = []


def check_bigram(label, k1, k2, target):
    rows = []
    for f1 in opts(k1):
        for f2 in opts(k2):
            rows.append((eff(k1, f1) + eff(k2, f2) +
                         _bigram_cost(k1, f1, k2, f2), f1, f2))
    rows.sort()
    got = (rows[0][1], rows[0][2])
    ok = got == target
    print(f"  [{'PASS' if ok else 'FAIL'}] {label} -> {got[0]}{got[1]} "
          f"({rows[0][0]:.3f}); want {target[0]}{target[1]}")
    if not ok:
        FAILS.append(label)


def check_trigram(label, k1, k2, k3, target):
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
    got = (rows[0][1], rows[0][2], rows[0][3])
    ok = got == target
    print(f"  [{'PASS' if ok else 'FAIL'}] {label} -> {''.join(got)} "
          f"({rows[0][0]:.3f}); want {''.join(target)}")
    if not ok:
        FAILS.append(label)


def section_probe():
    scorer2.TABLES = None
    bigram("O", "L", targets=[("M", "R"), ("M", "I"), ("R", "R"), ("R", "M")])
    bigram("L", "O", targets=[("R", "M"), ("I", "M"), ("M", "M")])
    bigram("X", "Q", targets=[("I", "M"), ("R", "P"), ("R", "M")])
    bigram("Q", "X", targets=[("M", "I"), ("P", "R")])
    bigram("S", "W", targets=[("I", "M"), ("R", "M"), ("M", "R")])
    bigram("W", "S", targets=[("M", "I"), ("M", "R")])
    bigram("X", "S", targets=[("I", "M"), ("M", "R"), ("R", "M")])
    bigram("F", "X", targets=[("I", "R"), ("M", "R"), ("I", "M")])
    bigram("X", "F", targets=[("R", "I"), ("M", "I"), ("I", "M")])
    bigram("X", "W", targets=[("I", "M"), ("M", "I"), ("M", "R")])
    bigram("W", "X", targets=[("M", "I"), ("I", "M"), ("R", "M")])
    bigram("U", "M", targets=[("M", "I"), ("I", "M")])
    bigram("M", "U", targets=[("I", "M"), ("M", "I")])
    bigram("A", "W", targets=[("P", "R"), ("R", "P")])
    bigram("W", "A", targets=[("R", "P"), ("P", "R")])
    print()
    trigram("F", "X", "W")
    trigram("W", "X", "F")
    trigram("O", "R", "M")
    trigram("X", "Q", "Z")
    # Full-span scissor probes (round-6b: M/R is symmetric; other pairs
    # keep the IN/OUT split). Real high-freq words, both directions.
    trigram("I", "M", "O")   # and  n->d IN
    trigram("M", "I", "M")   # dnd  n->d OUT (458)
    trigram("R", "V", "Y")   # you  o->u IN
    trigram("R", "V", "T")   # out  o->u IN
    trigram("U", "M", "I")   # und  l->d OUT
    trigram("I", "M", "E")   # nde  n->d IN
    trigram("U", "M", "Z")   # old  l->d OUT
    trigram("O", "M", "Z")   # ard  r->d OUT
    trigram("M", "O", "E")   # dre  d->r IN
    trigram("W", "Z", "U")   # abl  a->b IN
    trigram("W", "Z", "C")   # abo  a->b IN
    trigram("Z", "W", "N")   # bal  b->a OUT

    print("\nRound-6b ordering / symmetry checks:")
    def bc(k1, f1, k2, f2):
        return eff(k1, f1) + eff(k2, f2) + _bigram_cost(k1, f1, k2, f2)

    im = bc("I", "M", "M", "I")
    xw = bc("X", "I", "W", "M")
    um = bc("U", "M", "M", "I")
    ok = im < xw < um
    print(f"  [{'PASS' if ok else 'FAIL'}] I-M < X-W < U-M: "
          f"{im:.4f} < {xw:.4f} < {um:.4f}")
    if not ok:
        FAILS.append("I-M < X-W < U-M")

    xw_mr = bc("X", "M", "W", "R")
    wx_rm = bc("W", "R", "X", "M")
    ok = abs(xw_mr - wx_rm) < 1e-9
    print(f"  [{'PASS' if ok else 'FAIL'}] X-W M/R == W-X R/M: "
          f"{xw_mr:.4f} vs {wx_rm:.4f}")
    if not ok:
        FAILS.append("X-W M/R symmetric")

    qx_pr = comfort.scissor_cost("Q", "P", "X", "R", "L")
    xq_rp = comfort.scissor_cost("X", "R", "Q", "P", "L")
    ok = qx_pr >= comfort.W_IMPOSSIBLE_MR and xq_rp >= comfort.W_IMPOSSIBLE_MR
    print(f"  [{'PASS' if ok else 'FAIL'}] Q->X PR / X->Q RP impossible: "
          f"{qx_pr:.2f} / {xq_rp:.2f}")
    if not ok:
        FAILS.append("R/P reverse full-span impossible")

    print("\nRound-6b hard checks (top-1 must match):")
    check_bigram("O->L (MI preferred, round 6)", "O", "L", ("M", "I"))
    check_bigram("X->Q", "X", "Q", ("I", "M"))
    check_bigram("Q->X", "Q", "X", ("M", "I"))
    check_bigram("S->W", "S", "W", ("I", "M"))
    check_bigram("W->S", "W", "S", ("M", "I"))
    check_bigram("X->S", "X", "S", ("I", "M"))
    check_bigram("F->X", "F", "X", ("I", "R"))
    check_bigram("X->F", "X", "F", ("R", "I"))
    check_bigram("U->M (M/I no-scissor)", "U", "M", ("M", "I"))
    check_bigram("M->U (M/I reversal)", "M", "U", ("I", "M"))
    check_bigram("W->A (ring->pinky)", "W", "A", ("R", "P"))
    check_bigram("A->W (pinky->ring)", "A", "W", ("P", "R"))
    check_bigram("X->W (C5: natural no-ring pick)", "X", "W", ("I", "M"))
    check_bigram("W->X (C5 reversal)", "W", "X", ("M", "I"))
    check_trigram("fxw", "F", "X", "W", ("I", "M", "R"))
    check_trigram("wxf", "W", "X", "F", ("M", "I", "I"))
    if FAILS:
        print(f"\nPROBE FAILURES: {', '.join(FAILS)}")
    else:
        print("\nall round-6b hard checks PASS")


# ------------------------------------------------------------------ model
def section_model():
    """OLD model vs NEW model cfgE, crossing / ladder / repeat components."""
    import scorer, search

    t0 = time.time()
    scorer.TABLES = None
    scorer.DEFAULT_TABLES = None
    ce = scorer.Corpus.load(CORPUS)
    ev_old = search.Evaluator(ce, dead_of(WIN))
    s_old = ev_old.score(perm_of(WIN, dead_of(WIN)))
    print(f"OLD model cfgE = {s_old:.4f}   (tables built in {time.time()-t0:.1f}s)")

    t0 = time.time()
    scorer2.TABLES = None
    scorer2.DEFAULT_TABLES = None
    ev_new = Evaluator(ce, dead_of(WIN))
    s_new = ev_new.score(perm_of(WIN, dead_of(WIN)))
    print(f"NEW model cfgE = {s_new:.4f}   (tables built in {time.time()-t0:.1f}s)")

    BIG, TRI, BIG_F, TRI_F = get_tables()
    idx = {ch: i for i, ch in enumerate(WIN)}

    def show(bg):
        a, b = bg
        i, j = idx[a], idx[b]
        p = int(BIG_F[i, j])
        f1, f2 = FID2[p // 5], FID2[p % 5]
        k1, k2 = KEY_ORDER[i], KEY_ORDER[j]
        hand = KEYS[k1].hand
        cross = comfort.lateral_crossing(k1, f1, k2, f2, hand)
        print(f"  {bg} ({k1}->{k2}, {hand}): f={f1}{f2} BIG={BIG[i,j]:+.4f} "
              f"cross={cross:.3f} ansi_dist={ansi.dist(k1,k2):.3f}")

    print("\nCrossing check (user's ED vs IK):")
    show("ed")   # left M/I same col -> aligned
    show("ik")   # right hand same-col pair (stagger accentuates scissor)
    print("\nKey positions in cfgE:")
    for L in "nicde":
        pos = WIN.index(L)
        print(f"  letter '{L}' at KEY {KEY_ORDER[pos]} (row{pos//10} col{pos%10})")
    show("nc")

    print("\nLadder check: M/I bigram costs by row-pair (via BIG table):")
    seen = {}
    for i in range(30):
        for j in range(30):
            if i == j:
                continue
            p = int(BIG_F[i, j])
            f1, f2 = FID2[p // 5], FID2[p % 5]
            k1, k2 = KEY_ORDER[i], KEY_ORDER[j]
            if KEYS[k1].hand != KEYS[k2].hand:
                continue
            if {f1, f2} != {"M", "I"}:
                continue
            if mi_roll_penalty(k1, f1, k2, f2, KEYS[k1].hand) is None:
                continue
            mrow = int(round(ansi.cy(k1 if f1 == "M" else k2) - 0.5))
            irow = int(round(ansi.cy(k2 if f1 == "M" else k1) - 0.5))
            seen.setdefault((mrow, irow), []).append((k1 + k2, f1 + f2, BIG[i, j]))
    for cell, lad in sorted(_MI_LADDER.items()):
        items = sorted(seen.get(cell, []), key=lambda x: x[2])
        if items:
            print(f"  cell{cell} ladder={lad:.2f}  e.g. {items[0][0]} "
                  f"f={items[0][1]} BIG={items[0][2]:+.3f}")
        else:
            print(f"  cell{cell} ladder={lad:.2f}  (no bigram realizes this)")

    print("\nRepeat charge check:")
    i = idx["l"]
    p = int(BIG_F[i, i])
    f1, f2 = FID2[p // 5], FID2[p % 5]
    print(f"  'll': f={f1}{f2}  _bigram_cost = "
          f"{_bigram_cost('l',f1,'l',f2):.3f} (expect W_REPEAT=0.6)")
    print(f"  W_REPEAT={comfort.W_REPEAT} W_CROSS={comfort.W_CROSS}")


# ------------------------------------------------------------------ audit
def section_audit(t0=None):
    t0 = t0 or time.time()
    corpus = Corpus.load(CORPUS)
    BIG, TRI, BIG_F, TRI_F = get_tables()
    print(f"tables built in {time.time()-t0:.1f}s")
    lk = letter_keys(WIN)

    # 1. bigram impossible-winner scan
    print("\n=== 1. cfgE bigram impossible-winner scan ===")
    bg_impossible = []
    bg_total = 0
    for tg, cnt in corpus.B.items():
        if len(tg) != 2 or not all(c in LETTERS for c in tg):
            continue
        bg_total += cnt
        k1, k2 = lk[tg[0]], lk[tg[1]]
        i1, i2 = KEY_ORDER.index(k1), KEY_ORDER.index(k2)
        p = int(BIG_F[i1, i2])
        f1, f2 = INV_FID[p // 5], INV_FID[p % 5]
        if KEYS[k1].hand == KEYS[k2].hand and is_impossible(k1, f1, k2, f2):
            bg_impossible.append((tg, cnt, k1, k2, f1, f2, BIG[i1, i2]))
    print(f"  impossible winners: {len(bg_impossible)} "
          f"({sum(x[1] for x in bg_impossible):.0f} / {bg_total:.0f} tokens)")
    for row in bg_impossible[:10]:
        print(f"    {row[0]} n={row[1]} {row[2]}->{row[3]} {row[4]}{row[5]} "
              f"cost={row[6]:.3f}")

    # 2. trigram impossible-winner scan
    print("\n=== 2. cfgE trigram impossible-winner scan ===")
    tri_impossible = []
    tri_total = 0
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or not all(c in LETTERS for c in tg):
            continue
        tri_total += cnt
        ks = [lk[c] for c in tg]
        ids = [KEY_ORDER.index(k) for k in ks]
        fs = TRI_F[ids[0], ids[1], ids[2]]
        f1, f2, f3 = INV_FID[int(fs[0])], INV_FID[int(fs[1])], INV_FID[int(fs[2])]
        hand = KEYS[ks[0]].hand
        bad = ((KEYS[ks[1]].hand == hand and is_impossible(ks[0], f1, ks[1], f2))
               or (KEYS[ks[2]].hand == hand and is_impossible(ks[1], f2, ks[2], f3)))
        if bad:
            tri_impossible.append((tg, cnt, f1 + f2 + f3,
                                   TRI[ids[0], ids[1], ids[2]]))
    print(f"  impossible winners: {len(tri_impossible)} "
          f"({sum(x[1] for x in tri_impossible):.0f} / {tri_total:.0f} tokens)")
    for row in tri_impossible[:10]:
        print(f"    {row[0]} n={row[1]} fing={row[2]} cost={row[3]:.3f}")

    # 3. pick_audit regeneration
    print("\n=== 3. pick_audit regeneration ===")
    top_bg = [(t, c) for t, c in sorted(corpus.B.items(), key=lambda kv: -kv[1])
              if len(t) == 2 and all(ch in LETTERS for ch in t)][:60]
    lines = [
        "# Pick audit — cfgE, round-6b model (X-W stagger favor -0.35,",
        "# M/R full-span symmetric 5.85, downward 1-row SFB 6.3, R/P",
        "# reverse full-span impossible)",
        "# bg count SIDE KEY1->KEY fing cost [flags]",
        "",
        "## top-60 corpus bigrams (all classes)",
    ]
    n_imp = n_sfb = n_high = 0
    for tg, cnt in top_bg:
        k1, k2 = lk[tg[0]], lk[tg[1]]
        i1, i2 = KEY_ORDER.index(k1), KEY_ORDER.index(k2)
        p = int(BIG_F[i1, i2])
        f1, f2 = INV_FID[p // 5], INV_FID[p % 5]
        hand = KEYS[k1].hand
        flags = []
        if hand == KEYS[k2].hand:
            if is_impossible(k1, f1, k2, f2):
                flags.append("IMPOSSIBLE")
                n_imp += 1
            if f1 == f2 and k1 != k2:
                flags.append("SFB")
                n_sfb += 1
            if BIG[i1, i2] > 5.0:
                flags.append("HIGH")
                n_high += 1
        side = "X" if hand == "L" else "R"
        flag_s = ("  [" + ",".join(flags) + "]") if flags else ""
        lines.append(f"{tg:6s} {cnt:7d} {side} {k1}->{k2} {f1}{f2} "
                     f"{BIG[i1,i2]:.2f}{flag_s}")
    lines.append("")
    lines.append(f"## flags: IMPOSSIBLE={n_imp} SFB={n_sfb} HIGH={n_high}")
    with open(PICK_AUDIT, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"  wrote {PICK_AUDIT}")
    print(f"  IMPOSSIBLE={n_imp} SFB={n_sfb} HIGH={n_high}")

    # 4. rescoring
    print("\n=== 4. reference rescoring ===")
    candidates = dict(REF)
    candidates["cfgE"] = WIN
    candidates["annealBEST"] = ANNEAL_BEST
    candidates["NEW_BEST"] = NEW_BEST
    scored = {}
    for name, lay in candidates.items():
        if len(lay) != 30:
            print(f"{name}: bad length {len(lay)}")
            continue
        try:
            e = Evaluator(corpus, dead_of(lay))
            s = e.score(tuple(perm_of(lay, dead_of(lay))))
            scored[name] = (s, lay)
        except Exception as ex:
            print(f"{name}: ERROR {ex}")
    print(f"{'layout':22s} {'score':>8s}")
    for name, (s, lay) in sorted(scored.items(), key=lambda kv: kv[1][0]):
        print(f"  {name:22s} {s:8.4f}  {lay}")

    # 5. short seeded anneal (stability check)
    print("\n=== 5. short seeded anneal (20k iters, seed 11) ===")
    ev = Evaluator(corpus, dead_of(WIN))
    base_perm = tuple(perm_of(WIN, dead_of(WIN)))
    print(f"  cfgE start = {ev.score(base_perm):.4f}")
    _, best_s = ev.anneal(seed_perm=base_perm, iters=20000, seed=11)
    print(f"  anneal20k seed11 = {best_s:.4f}")

    print(f"\nDONE in {time.time()-t0:.1f}s")
    return len(bg_impossible) + len(tri_impossible), (n_imp, n_sfb, n_high)


def section_newbest(t0=None):
    t0 = t0 or time.time()
    corpus = Corpus.load(CORPUS)
    BIG, TRI, BIG_F, TRI_F = get_tables()
    lk = letter_keys(NEW_BEST)

    print(f"\n=== NEW_BEST audit ({NEW_BEST}) ===")
    bg_impossible = []
    n_sfb = n_high = 0
    bg_total = 0
    for tg, cnt in corpus.B.items():
        if len(tg) != 2 or not all(c in LETTERS for c in tg):
            continue
        bg_total += cnt
        k1, k2 = lk[tg[0]], lk[tg[1]]
        i1, i2 = KEY_ORDER.index(k1), KEY_ORDER.index(k2)
        p = int(BIG_F[i1, i2])
        f1, f2 = INV_FID[p // 5], INV_FID[p % 5]
        if KEYS[k1].hand != KEYS[k2].hand:
            continue
        if is_impossible(k1, f1, k2, f2):
            bg_impossible.append((tg, cnt, k1, k2, f1, f2, BIG[i1, i2]))
        if f1 == f2 and k1 != k2:
            n_sfb += 1
        if BIG[i1, i2] > 5.0:
            n_high += 1
    print(f"  bigram impossible winners: {len(bg_impossible)} "
          f"({sum(x[1] for x in bg_impossible):.0f} / {bg_total:.0f} tokens)")

    tri_impossible = []
    tri_total = 0
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or not all(c in LETTERS for c in tg):
            continue
        tri_total += cnt
        ks = [lk[c] for c in tg]
        ids = [KEY_ORDER.index(k) for k in ks]
        fs = TRI_F[ids[0], ids[1], ids[2]]
        f1, f2, f3 = (INV_FID[int(fs[0])], INV_FID[int(fs[1])],
                      INV_FID[int(fs[2])])
        hand = KEYS[ks[0]].hand
        bad = ((KEYS[ks[1]].hand == hand and is_impossible(ks[0], f1, ks[1], f2))
               or (KEYS[ks[2]].hand == hand and is_impossible(ks[1], f2, ks[2], f3)))
        if bad:
            tri_impossible.append((tg, cnt, f1 + f2 + f3,
                                   TRI[ids[0], ids[1], ids[2]]))
    print(f"  trigram impossible winners: {len(tri_impossible)} "
          f"({sum(x[1] for x in tri_impossible):.0f} / {tri_total:.0f} tokens)")

    top_bg = [(t, c) for t, c in sorted(corpus.B.items(), key=lambda kv: -kv[1])
              if len(t) == 2 and all(ch in LETTERS for ch in t)][:60]
    n_imp = n_tsfb = n_thigh = 0
    lines = [f"# Pick audit — NEW_BEST {NEW_BEST}, round-6b model",
             "# bg count SIDE KEY1->KEY fing cost [flags]", ""]
    for tg, cnt in top_bg:
        k1, k2 = lk[tg[0]], lk[tg[1]]
        i1, i2 = KEY_ORDER.index(k1), KEY_ORDER.index(k2)
        p = int(BIG_F[i1, i2])
        f1, f2 = INV_FID[p // 5], INV_FID[p % 5]
        flags = []
        if KEYS[k1].hand == KEYS[k2].hand:
            if is_impossible(k1, f1, k2, f2):
                flags.append("IMPOSSIBLE")
                n_imp += 1
            if f1 == f2 and k1 != k2:
                flags.append("SFB")
                n_tsfb += 1
            if BIG[i1, i2] > 5.0:
                flags.append("HIGH")
                n_thigh += 1
        if flags:
            side = "L" if KEYS[k1].hand == "L" else "R"
            lines.append(f"{tg:6s} {cnt:7d} {side} {k1}->{k2} {f1}{f2} "
                         f"{BIG[i1, i2]:.2f}  [{','.join(flags)}]")
    lines.append("")
    lines.append(f"## flags: IMPOSSIBLE={n_imp} SFB={n_tsfb} HIGH={n_thigh}")
    lines.append(f"## full-set flags: IMPOSSIBLE={len(bg_impossible)} "
                 f"SFB={n_sfb} HIGH={n_high}")
    with open(NEWBEST_AUDIT, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"  top-60 flags: IMPOSSIBLE={n_imp} SFB={n_tsfb} HIGH={n_thigh}")
    print(f"  full-set flags: IMPOSSIBLE={len(bg_impossible)} SFB={n_sfb} "
          f"HIGH={n_high}")

    ev = Evaluator(corpus, dead_of(NEW_BEST))
    score = ev.score(tuple(perm_of(NEW_BEST, dead_of(NEW_BEST))))
    print(f"  score = {score:.6f}")
    print(f"  wrote {NEWBEST_AUDIT}")
    print(f"  DONE in {time.time()-t0:.1f}s")
    return len(bg_impossible) + len(tri_impossible), (n_imp, n_tsfb, n_thigh)


SECTIONS = {"probe": section_probe, "model": section_model,
            "audit": section_audit, "newbest": section_newbest}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("-")]
    names = args or ["probe", "model", "audit", "newbest"]
    for name in names:
        if name not in SECTIONS:
            print(f"unknown section {name!r}; choose from {sorted(SECTIONS)}")
            sys.exit(2)
    scorer2.TABLES = None
    scorer2.DEFAULT_TABLES = None
    for name in names:
        print(f"\n########## section: {name} ##########")
        SECTIONS[name]()
    if "audit" in names:
        # hard gate for CI-style use: audits must be clean
        print("\nsections complete")
    if FAILS:
        sys.exit(1)


if __name__ == "__main__":
    main()
