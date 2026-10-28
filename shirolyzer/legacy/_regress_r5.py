#!/usr/bin/env python3
"""Round-5 full regression: impossible audits, pick_audit, reference rescoring."""
import sys
sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")

import time
from fingers import KEYS, KEY_ORDER
from comfort import (impossible_mr, impossible_ir, impossible_cross,
                     impossible_rp, scissor_cost, W_IMPOSSIBLE_MR)
from scorer2 import FID, get_tables, Corpus
from search2 import Evaluator, LETTERS
from compare import dead_of, REF

WIN = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
CORPUS = "/home/akai/git/layout-opencode-investigation/corpus/"
INV_FID = {v: k for k, v in FID.items()}


def is_impossible(k1, f1, k2, f2):
    return (impossible_mr(k1, f1, k2, f2)
            or impossible_ir(k1, f1, k2, f2)
            or impossible_cross(k1, f1, k2, f2)
            or impossible_rp(k1, f1, k2, f2))


def main():
    t0 = time.time()
    corpus = Corpus.load(CORPUS)
    BIG, TRI, BIG_F, TRI_F = get_tables()
    print(f"tables built in {time.time()-t0:.1f}s")

    # ---- 1. bigram impossible-winner scan (all letter bigrams under cfgE) ----
    print("\n=== 1. cfgE bigram impossible-winner scan ===")
    ev = Evaluator(corpus, dead_of(WIN))
    # map letters to keys under cfgE
    lay = WIN
    letter_key = {}
    for i, ch in enumerate(LETTERS):
        letter_key[ch] = KEY_ORDER[lay.index(ch)]

    bg_impossible = []
    bg_total = 0
    for tg, cnt in corpus.B.items():
        if len(tg) != 2 or not all(c in LETTERS for c in tg):
            continue
        bg_total += cnt
        k1, k2 = letter_key[tg[0]], letter_key[tg[1]]
        # decode winning fingering from BIG_F
        i1, i2 = KEY_ORDER.index(k1), KEY_ORDER.index(k2)
        p = int(BIG_F[i1, i2])
        f1, f2 = INV_FID[p // 5], INV_FID[p % 5]
        if KEYS[k1].hand == KEYS[k2].hand and is_impossible(k1, f1, k2, f2):
            bg_impossible.append((tg, cnt, k1, k2, f1, f2, BIG[i1, i2]))
    print(f"  bigram types scanned, impossible winners: {len(bg_impossible)} "
          f"({sum(x[1] for x in bg_impossible):.0f} / {bg_total:.0f} tokens)")
    for row in bg_impossible[:10]:
        print(f"    {row[0]} n={row[1]} {row[2]}->{row[3]} {row[4]}{row[5]} "
              f"cost={row[6]:.3f}")

    # ---- 2. trigram impossible-winner scan ----
    print("\n=== 2. cfgE trigram impossible-winner scan ===")
    tri_impossible = []
    tri_total = 0
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or not all(c in LETTERS for c in tg):
            continue
        tri_total += cnt
        ks = [letter_key[c] for c in tg]
        ids = [KEY_ORDER.index(k) for k in ks]
        fs = TRI_F[ids[0], ids[1], ids[2]]
        f1, f2, f3 = INV_FID[int(fs[0])], INV_FID[int(fs[1])], INV_FID[int(fs[2])]
        hand = KEYS[ks[0]].hand
        bad = False
        if KEYS[ks[1]].hand == hand and is_impossible(ks[0], f1, ks[1], f2):
            bad = True
        if KEYS[ks[2]].hand == hand and is_impossible(ks[1], f2, ks[2], f3):
            bad = True
        if bad:
            tri_impossible.append((tg, cnt, ks, f1 + f2 + f3,
                                   TRI[ids[0], ids[1], ids[2]]))
    print(f"  trigram types scanned, impossible winners: {len(tri_impossible)} "
          f"({sum(x[1] for x in tri_impossible):.0f} / {tri_total:.0f} tokens)")
    for row in tri_impossible[:10]:
        print(f"    {row[0]} n={row[1]} fing={row[3]} cost={row[4]:.3f}")

    # ---- 3. pick_audit: top-60 bigrams with flags ----
    print("\n=== 3. pick_audit regeneration ===")
    top_bg = sorted(corpus.B.items(), key=lambda kv: -kv[1])
    top_bg = [(t, c) for t, c in top_bg
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
        k1, k2 = letter_key[tg[0]], letter_key[tg[1]]
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
        lines.append(
            f"{tg:6s} {cnt:7d} {side} {k1}->{k2} {f1}{f2} "
            f"{BIG[i1,i2]:.2f}{flag_s}")
    lines.append("")
    lines.append(f"## flags: IMPOSSIBLE={n_imp} SFB={n_sfb} HIGH={n_high}")
    outp = "/home/akai/git/layout-opencode-investigation/results/pick_audit.txt"
    with open(outp, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"  wrote {outp}")
    print(f"  IMPOSSIBLE={n_imp} SFB={n_sfb} HIGH={n_high}")

    # ---- 4. rescore references + cfgE + prior anneal best ----
    print("\n=== 4. reference rescoring ===")
    candidates = dict(REF)
    candidates["cfgE"] = WIN
    candidates["annealBEST"] = ";stckzdaiwnlprhyeguobfvmjqx,./"
    print(f"{'layout':22s} {'score':>8s}")
    scored = {}
    for name, lay in candidates.items():
        if len(lay) != 30:
            print(f"{name}: bad length {len(lay)}")
            continue
        try:
            e = Evaluator(corpus, dead_of(lay))
            perm = []
            slot_pos = {k: i for i, k in enumerate(e.slots)}
            for ch in LETTERS:
                perm.append(slot_pos[KEY_ORDER[lay.index(ch)]])
            s = e.score(tuple(perm))
            scored[name] = (s, lay)
        except Exception as ex:
            print(f"{name}: ERROR {ex}")
    for name, (s, lay) in sorted(scored.items(), key=lambda kv: kv[1][0]):
        print(f"  {name:22s} {s:8.4f}  {lay}")

    # ---- 5. short seeded anneal from cfgE (stability check) ----
    print("\n=== 5. short seeded anneal (20k iters, seed 11) ===")
    ev = Evaluator(corpus, dead_of(WIN))
    base_perm = []
    slot_pos = {k: i for i, k in enumerate(ev.slots)}
    for ch in LETTERS:
        base_perm.append(slot_pos[KEY_ORDER[WIN.index(ch)]])
    base_s = ev.score(tuple(base_perm))
    print(f"  cfgE start = {base_s:.4f}")
    best_perm, best_s = ev.anneal(seed_perm=base_perm, iters=20000, seed=11)
    best_lay = "".join(ev.slots[i] for i in best_perm)
    # map slots back through dead-key layout... slots are physical keys without dead
    # reconstruct: for each letter position, which slot
    print(f"  anneal20k seed11 = {best_s:.4f}")

    print(f"\nDONE in {time.time()-t0:.1f}s")


if __name__ == "__main__":
    main()
