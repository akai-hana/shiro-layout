"""
metrics.py — Detailed layout diagnostics: per-bigram/trigram category stats
under context-optimal alt-fingering (and under default fingering for
comparison with standard analyzers).
"""
import numpy as np
from fingers import (KEYS, KEY_ORDER, SPACE, FORDER, scissor_cost, cheb,
                     IDEAL_SHARE, REST)
from scorer import get_tables, get_default_tables, SPACE_ID, N

FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}


def _fset(name):
    if name == SPACE:
        return [("T", 0.0)]
    return list(KEYS[name].alts)


def analyze_bigram_counts(layout30, bigram_counts, with_alts=True):
    """Return category-weighted stats for letter bigrams (no space).

    Categories (chosen fingering): sfb, scissors (typed by pair+direction),
    roll_in, roll_out, alt (cross-hand), leap, repeat.
    """
    m = {ch: KEY_ORDER[i] for i, ch in enumerate(layout30)}
    BIG, TRI, BIG_F, TRI_F = get_tables()
    stats = {
        "total": 0, "sfb": 0.0, "roll_in": 0.0, "roll_out": 0.0,
        "alt": 0.0, "leap": 0.0, "repeat": 0.0,
        "scissor_IM_hsb": 0.0, "scissor_IM_fsb": 0.0,
        "scissor_MR": 0.0, "scissor_RP": 0.0,
        "per_bigram": {},
    }
    worst = []
    for bg, cnt in bigram_counts.items():
        if len(bg) != 2 or bg[0] not in m or bg[1] not in m:
            continue
        n1, n2 = m[bg[0]], m[bg[1]]
        i, j = KEY_ORDER.index(n1) if n1 != SPACE else SPACE_ID, \
               KEY_ORDER.index(n2) if n2 != SPACE else SPACE_ID
        cost = BIG[i, j]
        packed = int(BIG_F[i, j])
        f1, f2 = FID2[packed // 5], FID2[packed % 5]
        stats["total"] += cnt
        stats["per_bigram"][bg] = (cost, f1, f2, cnt)
        worst.append((cost, bg, f1, f2, cnt))
        if n1 == n2:
            stats["repeat"] += cnt
        elif KEYS[n1].hand == KEYS[n2].hand and f1 == f2:
            stats["sfb"] += cnt
        elif KEYS[n1].hand != KEYS[n2].hand:
            stats["alt"] += cnt
        else:
            o1, o2 = FORDER[f1], FORDER[f2]
            dcol = abs(KEYS[n1].col - KEYS[n2].col)
            if abs(o2 - o1) >= 2 or dcol >= 3:
                stats["leap"] += cnt
            elif o2 > o1:
                stats["roll_in"] += cnt
            else:
                stats["roll_out"] += cnt
            # scissors by pair
            sc = scissor_cost(n1, f1, n2, f2)
            if sc > 0:
                dist = cheb((KEYS[n1].row, KEYS[n1].col),
                            (KEYS[n2].row, KEYS[n2].col))
                outer = f1 if FORDER[f1] < FORDER[f2] else f2
                inner = f2 if outer == f1 else f1
                if (outer, inner) == ("M", "I"):
                    if dist == 1:
                        stats["scissor_IM_hsb"] += cnt
                    else:
                        stats["scissor_IM_fsb"] += cnt
                elif (outer, inner) == ("R", "M"):
                    stats["scissor_MR"] += cnt
                elif (outer, inner) == ("P", "R"):
                    stats["scissor_RP"] += cnt
    worst.sort(reverse=True)
    return stats, worst


def category_report(layout30, corpus):
    stats, worst = analyze_bigram_counts(layout30, corpus.B)
    tot = stats["total"] or 1
    rep = {k: (v / tot * 100 if k != "total" and k != "per_bigram" else v)
           for k, v in stats.items()}
    # default-fingering SFB for comparison
    m = {ch: KEY_ORDER[i] for i, ch in enumerate(layout30)}
    sfb_def = 0.0
    sfb_alt_only = 0.0
    for bg, cnt in corpus.B.items():
        if len(bg) != 2 or bg[0] not in m or bg[1] not in m:
            continue
        n1, n2 = m[bg[0]], m[bg[1]]
        f1d = KEYS[n1].alts[0][0]
        f2d = KEYS[n2].alts[0][0]
        if n1 != n2 and f1d == f2d and KEYS[n1].hand == KEYS[n2].hand:
            sfb_def += cnt
        # alt-removable?
        i = KEY_ORDER.index(n1)
        j = KEY_ORDER.index(n2)
        packed = int(get_tables()[2][i, j])
        f1a, f2a = FID2[packed // 5], FID2[packed % 5]
        if n1 != n2 and f1a == f2a and KEYS[n1].hand == KEYS[n2].hand:
            sfb_alt_only += cnt
    rep["sfb_default_pct"] = sfb_def / tot * 100
    rep["sfb_unavoidable_pct"] = sfb_alt_only / tot * 100
    rep["worst_bigrams"] = worst[:25]
    return rep


def usage_report(layout30, corpus, use_dp=True):
    """Finger/hand usage %. If use_dp, weight by DP-optimal finger choice
    over bigrams; else use default fingers over letter freq."""
    m = {ch: KEY_ORDER[i] for i, ch in enumerate(layout30)}
    BIG_F = get_tables()[2]
    usage = {h: {f: 0.0 for f in "PRMI"} for h in "LR"}
    if use_dp:
        for bg, cnt in corpus.B.items():
            if len(bg) != 2 or bg[0] not in m or bg[1] not in m:
                continue
            for ch in bg:
                k = m[ch]
                i = KEY_ORDER.index(k)
                # majority-vote over contexts is expensive; use default here
        # simpler: default-finger letter frequency (stable, deterministic)
        for ch, cnt in corpus.L.items():
            if ch not in m:
                continue
            k = m[ch]
            f = KEYS[k].alts[0][0]
            usage[KEYS[k].hand][f] += cnt
    else:
        for ch, cnt in corpus.L.items():
            if ch not in m:
                continue
            k = m[ch]
            f = KEYS[k].alts[0][0]
            usage[KEYS[k].hand][f] += cnt
    tot = sum(sum(usage[h].values()) for h in "LR") or 1
    out = {}
    for h in "LR":
        hs = sum(usage[h].values())
        for f in "PRMI":
            out[f"{h}{f}"] = usage[h][f] / tot * 100
    out["handL"] = sum(usage["L"].values()) / tot * 100
    out["handR"] = sum(usage["R"].values()) / tot * 100
    return out


def trigram_pattern_report(layout30, corpus):
    """Community-style trigram patterns (oxeylyzer/layouts.wiki vocabulary):
    - alternate: strict hand alternation L-R-L / R-L-R
    - roll: two consecutive strokes on one hand, third switches
      (LLR/RLL/RRL/LRR); inroll = the same-hand pair moves toward index
    - onehand: all three strokes same hand, no SFB, no redirect
    - redirect: same-hand, direction reverses (weak = no index involved)
    - sfb_tri: same-hand trigram containing an adjacent same-finger pair
    Space excluded (corpus is within-word)."""
    m = {ch: KEY_ORDER[i] for i, ch in enumerate(layout30)}
    acc = {"inroll": 0.0, "outroll": 0.0, "alternate": 0.0, "onehand": 0.0,
           "redirect": 0.0, "redirect_weak": 0.0, "sfb_tri": 0.0, "other": 0.0}
    tot = 0.0
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or any(c not in m for c in tg):
            continue
        n1, n2, n3 = (m[c] for c in tg)
        ks = (KEYS[n1], KEYS[n2], KEYS[n3])
        hs = tuple(k.hand for k in ks)
        fs = tuple(KEYS[n].alts[0][0] for n in (n1, n2, n3))
        os_ = tuple(FORDER[f] for f in fs)
        tot += cnt

        if hs in (("L", "R", "L"), ("R", "L", "R")):
            acc["alternate"] += cnt
            continue

        if hs[0] == hs[1] == hs[2]:
            if fs[0] == fs[1] or fs[1] == fs[2]:
                acc["sfb_tri"] += cnt
                continue
            d1, d2 = os_[1] - os_[0], os_[2] - os_[1]
            if d1 * d2 < 0:
                if "I" in fs:
                    acc["redirect"] += cnt
                else:
                    acc["redirect_weak"] += cnt
            else:
                acc["onehand"] += cnt
            continue

        # roll: LLR / RLL / RRL / LRR — same-hand pair must not be SFB
        if hs[0] == hs[1]:
            if fs[0] == fs[1]:
                acc["sfb_tri"] += cnt
            elif os_[0] < os_[1]:
                acc["inroll"] += cnt
            else:
                acc["outroll"] += cnt
        elif hs[1] == hs[2]:
            if fs[1] == fs[2]:
                acc["sfb_tri"] += cnt
            elif os_[1] < os_[2]:
                acc["inroll"] += cnt
            else:
                acc["outroll"] += cnt
        else:
            acc["other"] += cnt

    tot = tot or 1.0
    out = {k: v / tot * 100 for k, v in acc.items()}
    out["total_rolls"] = out["inroll"] + out["outroll"]
    out["total_redirects"] = out["redirect"] + out["redirect_weak"]
    return out


def print_grid(layout30):
    top, home, bot = layout30[:10], layout30[10:20], layout30[20:30]
    print("        L hand           R hand")
    print(f"top :  {' '.join(top[:5])}  |  {' '.join(top[5:])}")
    print(f"home:  {' '.join(home[:5])}  |  {' '.join(home[5:])}")
    print(f"bot :  {' '.join(bot[:5])}  |  {' '.join(bot[5:])}")
    print("rest:  A W E F     J I O ;   (pinky ring mid idx | idx mid ring pinky)")
