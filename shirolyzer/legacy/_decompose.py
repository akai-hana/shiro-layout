#!/usr/bin/env python3
"""Decompose v2-model costs by category: cfgE vs new-model winners.

Mirrors scorer2 v2 semantics (leap tiers, pinky rolls/redirects, awful bonus,
deadzone crossing, scaled travel) using the min-cost fingers from BIG_F/TRI_F.
"""
import sys
sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from collections import defaultdict

WIN = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
OLD_BEST = ";rolxzciakthmsfpnyuedjgvqbw,./"
NEW_BEST = ";stpbqdaiynhcrmkeguowvflzxj,./"

from scorer2 import get_tables, W_REPEAT, W_REPEAT_SWAP
from scorer import Corpus
from fingers import KEY_ORDER, KEYS, FORDER
from comfort import (scissor_cost, lateral_crossing, stagger_favor,
                     mi_roll_penalty,
                     awful_trigram_flags, AWFUL_THRESHOLD, W_AWFUL_TRIGRAM,
                     W_SFB_SAME_ROW, W_SFB_1ROW, W_SFB_1ROW_DOWN,
                     W_SFB_2ROW, W_ALT,
                     W_ROLL_IN, W_ROLL_OUT, W_ROLL_MI_SYM,
                     W_ROLL_RING_PINKY, W_ROLL_PINKY_RING,
                     W_ROLL_LEAP_GAP2, W_ROLL_LEAP_GAP3,
                     PINKY_LEAP_EXTRA, W_FULLSPAN_SCISSOR_STRAIN,
                     is_full_span_scissor,
                     W_REDIRECT, W_REDIRECT_WEAK, W_CONTINUE,
                     W_SFS_1U, W_SFS_FAR, FLAT_SHARE, W_BALANCE_EVEN,
                     W_HAND_BALANCE, W_ROLL_WIDE_GAP)
import ansi

BIG, TRI, BIG_F, TRI_F = get_tables()
FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}
ce = Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")
tot_b = sum(ce.B.values())
tot_t = sum(ce.T.values())


def bigram_parts(n1, f1, n2, f2):
    """Category -> cost for one chosen-finger bigram (mirrors scorer2 v2)."""
    out = defaultdict(float)
    if n1 == n2:
        out["repeat"] += W_REPEAT + (W_REPEAT_SWAP if f1 != f2 else 0.0)
        return out
    k1, k2 = KEYS[n1], KEYS[n2]
    if k1.hand != k2.hand:
        out["alt"] += W_ALT
        return out
    if f1 == f2:
        dr = abs(ansi.row(n1) - ansi.row(n2))
        if dr == 0:
            b = W_SFB_SAME_ROW
        elif dr == 1:
            b = W_SFB_1ROW_DOWN if ansi.row(n2) > ansi.row(n1) else W_SFB_1ROW
        else:
            b = W_SFB_2ROW
        out["sfb"] += b
        return out
    sc = scissor_cost(n1, f1, n2, f2, k1.hand)
    cross = lateral_crossing(n1, f1, n2, f2, k1.hand)
    o1, o2 = FORDER[f1], FORDER[f2]
    pair = {f1, f2}
    full_sc = is_full_span_scissor(n1, n2, sc)
    if pair == {"M", "I"}:
        v = mi_roll_penalty(n1, f1, n2, f2, k1.hand)
        if v:
            out["ladder"] += v
    if abs(o2 - o1) >= 2:
        base = W_ROLL_LEAP_GAP3 if abs(o2 - o1) >= 3 else W_ROLL_LEAP_GAP2
        reward = base + (PINKY_LEAP_EXTRA if "P" in (f1, f2) else 0.0)
        cat = "leap"
    else:
        if pair == {"M", "I"}:
            reward = W_ROLL_MI_SYM
        elif pair == {"P", "R"}:
            reward = W_ROLL_RING_PINKY if f2 == "P" else W_ROLL_PINKY_RING
        else:
            reward = W_ROLL_IN if o2 > o1 else W_ROLL_OUT
        if ansi.dist(n1, n2) >= W_ROLL_WIDE_GAP:
            reward *= 0.5
        if cross > 0:
            reward *= 0.25
        cat = "roll"
    if pair == {"M", "R"} and full_sc:
        reward = 0.0
        cat = "roll"
    elif o2 > o1 and pair != {"M", "I"} and full_sc:
        reward = W_FULLSPAN_SCISSOR_STRAIN
        cat = "roll"
    out[cat] += reward
    sf = stagger_favor(n1, f1, n2, f2, k1.hand)
    if sf:
        out["stagger"] -= sf
    if sc:
        out["scissor"] += sc
    if cross:
        out["cross"] += cross
    return out


def parts_of(lay):
    idx = {ch: i for i, ch in enumerate(lay)}
    cat = defaultdict(float)
    for bg, cnt in ce.B.items():
        if len(bg) != 2 or any(c not in idx for c in bg):
            continue
        i, j = idx[bg[0]], idx[bg[1]]
        p = int(BIG_F[i, j]); f1, f2 = FID2[p // 5], FID2[p % 5]
        n1, n2 = KEY_ORDER[i], KEY_ORDER[j]
        w = cnt / tot_b
        for k, v in bigram_parts(n1, f1, n2, f2).items():
            cat[k] += w * v
    for tg, cnt in ce.T.items():
        if len(tg) != 3 or any(c not in idx for c in tg):
            continue
        i, j, k = idx[tg[0]], idx[tg[1]], idx[tg[2]]
        f = TRI_F[i, j, k]
        f1, f2, f3 = FID2[int(f[0])], FID2[int(f[1])], FID2[int(f[2])]
        n1, n2, n3 = KEY_ORDER[i], KEY_ORDER[j], KEY_ORDER[k]
        w = cnt / tot_t
        if f1 == f3 and f2 != f1 and KEYS[n1].hand == KEYS[n3].hand:
            d = ansi.dist(n1, n3)
            cat["sfs"] += w * (W_SFS_1U if d <= 1.3 else W_SFS_FAR)
        ks = (KEYS[n1], KEYS[n2], KEYS[n3])
        if ks[0].hand != ks[1].hand or ks[1].hand != ks[2].hand:
            continue
        o = (FORDER[f1], FORDER[f2], FORDER[f3])
        d1, d2 = o[1] - o[0], o[2] - o[1]
        if d1 == 0 or d2 == 0:
            continue
        if d1 * d2 < 0:
            pinky = "P" in (f1, f2, f3)
            cat["redirect_pinky" if pinky else "redirect"] += w * (
                W_REDIRECT_WEAK if pinky else W_REDIRECT)
            if awful_trigram_flags(n1, f1, n2, f2, n3, f3) >= AWFUL_THRESHOLD:
                cat["awful"] += w * W_AWFUL_TRIGRAM
        else:
            cat["continue"] += w * W_CONTINUE
    return cat


def balance_of(lay, ev):
    """Exact balance penalty from search2 semantics."""
    from compare import dead_of
    dead = dead_of(lay)
    slots = [k for k in KEY_ORDER if k not in dead.values()]
    slot_of = {k: i for i, k in enumerate(slots)}
    perm = [slot_of[KEY_ORDER[lay.index(ch)]] for ch in "abcdefghijklmnopqrstuvwxyz"]
    kid = ev._key_ids(perm)
    LETTERS = "abcdefghijklmnopqrstuvwxyz"
    usage = {"L": [0.0] * 4, "R": [0.0] * 4}
    fidx = {"P": 0, "R": 1, "M": 2, "I": 3}
    lfreq = [ev.lfreq[i] for i in range(26)]
    for li in range(26):
        kk = KEY_ORDER[kid[li]]
        f = KEYS[kk].alts[0][0]
        usage[KEYS[kk].hand][fidx[f]] += lfreq[li]
    pen = 0.0
    for h in "LR":
        hs = sum(usage[h])
        for fi in range(4):
            share = usage[h][fi] / hs
            ideal = FLAT_SHARE[["P", "R", "M", "I"][fi]]
            pen += ((share - ideal) / ideal) ** 2
    pen *= W_BALANCE_EVEN
    hl, hr = sum(usage["L"]), sum(usage["R"])
    pen += ((hl - hr) / (hl + hr)) ** 2 * W_HAND_BALANCE
    return pen, usage


if __name__ == "__main__":
    import search2
    from compare import dead_of
    ce2 = search2.Corpus if False else None
    import scorer2 as S2
    corpus = S2.Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")
    ev = search2.Evaluator(corpus, dead_of(WIN))

    def perm_of(lay):
        dead = dead_of(WIN)
        slots = [k for k in KEY_ORDER if k not in dead.values()]
        slot_of = {k: i for i, k in enumerate(slots)}
        return [slot_of[KEY_ORDER[lay.index(ch)]] for ch in "abcdefghijklmnopqrstuvwxyz"]

    layouts = {"cfgE": WIN, "OLD-BEST": OLD_BEST, "NEW-BEST": NEW_BEST}
    cats = {nm: parts_of(lay) for nm, lay in layouts.items()}
    keys = sorted({k for c in cats.values() for k in c})
    print(f"{'category':16s} {'cfgE':>9s} {'oldBEST':>9s} {'newBEST':>9s} {'d(new-cfgE)':>12s}")
    print("-" * 60)
    tot = {}
    for nm in layouts:
        tot[nm] = sum(cats[nm].values())
    for k in keys:
        row = [cats[nm].get(k, 0.0) for nm in ("cfgE", "OLD-BEST", "NEW-BEST")]
        mark = "  <<<" if abs(row[2] - row[0]) > 0.01 else ""
        print(f"{k:16s} {row[0]:+9.4f} {row[1]:+9.4f} {row[2]:+9.4f} {row[2]-row[0]:+12.4f}{mark}")
    print("-" * 60)
    print(f"{'TRANSITION tot':16s} {tot['cfgE']:+9.4f} {tot['OLD-BEST']:+9.4f} {tot['NEW-BEST']:+9.4f} {tot['NEW-BEST']-tot['cfgE']:+12.4f}")
    for nm, lay in layouts.items():
        sc = ev.score(perm_of(lay))
        bal, usage = balance_of(lay, ev)
        eff = sc - bal - tot[nm]
        print(f"{nm:10s} score={sc:.4f}  balance={bal:.4f}  effort(residual)={eff:.4f}")
        for h in "LR":
            hs = sum(usage[h])
            shares = [usage[h][i] / hs for i in range(4)]
            print(f"    {h} shares P/R/M/I = {shares[0]:.3f}/{shares[1]:.3f}/{shares[2]:.3f}/{shares[3]:.3f}  target "
                  f"{FLAT_SHARE['P']:.2f}/{FLAT_SHARE['R']:.2f}/{FLAT_SHARE['M']:.2f}/{FLAT_SHARE['I']:.2f}")
