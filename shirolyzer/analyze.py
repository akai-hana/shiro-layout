#!/usr/bin/env python3
"""Analyze a layout with the calibrated pattern model.

Usage:
    python3 shirolyzer/analyze.py --layout ";tspbxgoiwndhrlyecuafkvmzjq,./" \
        --primary-corpus corpora/shai/parsed
"""
import argparse
import json
from collections import defaultdict
from pathlib import Path

import numpy as np

from utils import comfort as cg
from utils import paths
from utils import scorer as SG
from utils.corpus import load_many
from utils.ansi import KEYS, KI, H, CX, FI, ROWI
from utils.fingers import DEFAULT_FINGER

LETTERS = "abcdefghijklmnopqrstuvwxyz"
FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}
FORDER = {"P": 0, "R": 1, "M": 2, "I": 3}
DEFAULT_LAYOUT = ";tspbxgoiwndhrlyecuafkvmzjq,./"


def keymap(layout):
    return {ch: KEYS[i] for i, ch in enumerate(layout) if ch.isalpha()}


def slots_of(layout):
    dead = {ch: KEYS[i] for i, ch in enumerate(layout) if ch not in LETTERS}
    slots = [k for k in KEYS if k not in dead.values()]
    if len(slots) != 26:
        raise SystemExit(f"layout must leave 26 letter slots, got {len(slots)}")
    pos = {k: i for i, k in enumerate(slots)}
    return tuple(pos[layout[i]] for i, ch in enumerate(layout) if ch in LETTERS), dead


def score_layout(layout, corpus, tables=None):
    big, tri, _, _ = tables or SG.get_tables()
    m = keymap(layout)
    remap = np.full(31, -1, dtype=np.int32)
    for ci, ch in enumerate(corpus.chars):
        if ch == SG.SPACE:
            remap[ci] = SG.SPACE_ID
        elif ch in m:
            remap[ci] = KI[m[ch]]
    tri_ids = remap[corpus.tri_ids]
    ok = np.all(tri_ids >= 0, axis=1)
    flat = tri_ids[ok, 0] * 31 * 31 + tri_ids[ok, 1] * 31 + tri_ids[ok, 2]
    total = float(corpus.tri_counts[ok].sum()) or 1.0
    return float((tri.ravel()[flat] * corpus.tri_counts[ok]).sum() / total)


def category_report(layout, corpus, tables):
    m = keymap(layout)
    big, tri, bigf, _ = tables
    stats = defaultdict(float)
    total = 0.0
    worst, scissors = [], []
    for bg, cnt in corpus.B.items():
        if len(bg) != 2 or bg[0] not in m or bg[1] not in m:
            continue
        total += cnt
        a, b = bg[0], bg[1]
        i, j = KI[m[a]], KI[m[b]]
        cost = float(big[i, j])
        enc = int(bigf[i, j])
        f1, f2 = FID2[enc // 5], FID2[enc % 5]
        worst.append((cost, bg, f1, f2, cnt))
        if m[a] == m[b]:
            stats["repeat"] += cnt
        elif H[i] != H[j]:
            stats["alt"] += cnt
        elif f1 == f2:
            stats["sfb"] += cnt
        else:
            d1, d2 = FORDER[f1], FORDER[f2]
            dcol = abs(CX[i] - CX[j])
            if abs(d2 - d1) >= 2 or dcol >= 3:
                stats["leap"] += cnt
            elif d2 > d1:
                stats["roll_in"] += cnt
            else:
                stats["roll_out"] += cnt
            g = float(cg.PF["g"][i, FI[f1], j, FI[f2]])
            if g > 0.25:
                outer, inner = (f1, f2) if FORDER[f1] < FORDER[f2] else (f2, f1)
                kind = {"M": "scissor_IM", "R": "scissor_MR", "P": "scissor_RP"}.get(inner)
                if kind:
                    stats[kind + ("_fsb" if abs(ROWI[i] - ROWI[j]) >= 2 else "_hsb")] += cnt
                    scissors.append((cost * cnt, cost, bg, f1, f2, cnt, outer + inner))
    total = total or 1.0
    cats = {k: stats[k] / total * 100 for k in (
        "sfb", "scissor_IM_hsb", "scissor_IM_fsb", "scissor_MR", "scissor_RP",
        "roll_in", "roll_out", "leap", "alt", "repeat")}
    cats["total_tokens"] = total
    worst.sort(reverse=True)
    scissors.sort(reverse=True)
    return cats, worst[:30], scissors[:20]


def trigram_patterns(layout, corpus, tables):
    m = keymap(layout)
    trif = tables[3]
    acc = defaultdict(float)
    total = 0.0
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or any(c not in m for c in tg):
            continue
        ids = [KI[m[c]] for c in tg]
        fs = tuple(FID2[int(x)] for x in trif[ids[0], ids[1], ids[2]])
        hs = tuple(int(H[k]) for k in ids)
        os_ = tuple(FORDER[f] for f in fs)
        total += cnt
        if hs in ((0, 1, 0), (1, 0, 1)):
            acc["alternate"] += cnt
        elif hs[0] == hs[1] == hs[2]:
            if fs[0] == fs[1] or fs[1] == fs[2]:
                acc["sfb_tri"] += cnt
            elif (os_[1] - os_[0]) * (os_[2] - os_[1]) < 0:
                acc["redirect" if "I" in fs else "redirect_weak"] += cnt
            else:
                acc["onehand"] += cnt
        elif hs[0] == hs[1]:
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
    total = total or 1.0
    out = {k: v / total * 100 for k, v in acc.items()}
    out["total_rolls"] = out.get("inroll", 0) + out.get("outroll", 0)
    out["total_redirects"] = out.get("redirect", 0) + out.get("redirect_weak", 0)
    return out


def finger_usage(layout, corpus, tables):
    m = keymap(layout)
    trif = tables[3]
    usage = {0: defaultdict(float), 1: defaultdict(float)}
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or any(c not in m for c in tg):
            continue
        ids = [KI[m[c]] for c in tg]
        fs = trif[ids[0], ids[1], ids[2]]
        for k, fid in zip(ids, fs):
            usage[int(H[k])][FID2[int(fid)]] += cnt
    total = sum(sum(h.values()) for h in usage.values()) or 1.0
    out = {}
    for h, label in ((0, "L"), (1, "R")):
        for f in "PRMI":
            out[label + f] = usage[h][f] / total * 100
    out["handL"] = sum(usage[0].values()) / total * 100
    out["handR"] = sum(usage[1].values()) / total * 100
    return out


def fingering_guide(layout, corpus, tables):
    m = keymap(layout)
    trif = tables[3]
    agg = {ch: defaultdict(float) for ch in LETTERS}
    ctx = {ch: defaultdict(lambda: defaultdict(float)) for ch in LETTERS}
    for tg, cnt in corpus.T.items():
        if len(tg) != 3 or any(c not in m for c in tg):
            continue
        ids = [KI[m[c]] for c in tg]
        fs = [FID2[int(x)] for x in trif[ids[0], ids[1], ids[2]]]
        for pos, ch in enumerate(tg):
            agg[ch][fs[pos]] += cnt
            ctx[ch][fs[pos]][tg] += cnt
    guide = {}
    for ch in LETTERS:
        if ch not in m:
            continue
        key = m[ch]
        default = DEFAULT_FINGER[key]
        total = sum(agg[ch].values()) or 1.0
        entry = {"key": key, "default": default,
                 "share": {f: c / total * 100 for f, c in sorted(agg[ch].items(), key=lambda x: -x[1])},
                 "alt_contexts": {}}
        for f in entry["share"]:
            if f == default:
                continue
            entry["alt_contexts"][f] = [
                (tg, round(c)) for tg, c in sorted(ctx[ch][f].items(), key=lambda x: -x[1])[:8]]
        guide[ch] = entry
    return guide


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--layout", default=DEFAULT_LAYOUT)
    ap.add_argument("--name", default=None,
                    help="layout name used to derive default output paths")
    ap.add_argument("--primary-corpus", action="append", default=[],
                    help="parsed corpus dir, corpus dir containing parsed/, or raw .txt/.xz/.zip (repeatable)")
    ap.add_argument("--secondary-corpus", action="append", default=[],
                    help="parsed corpus dir, corpus dir containing parsed/, or raw .txt/.xz/.zip (repeatable)")
    ap.add_argument("--out", default=None)
    ap.add_argument("--out-json", default=None)
    args = ap.parse_args()
    if len(args.layout) != 30:
        raise SystemExit("layout must be 30 characters")
    out_path = Path(args.out) if args.out else paths.result_path("analysis", args.name)
    out_json = (Path(args.out_json) if args.out_json
                else paths.result_path("analysis", args.name, suffix=".json"))
    primary_paths = (args.primary_corpus
                     or [str(paths.CORPUS_ROOT / "shai" / "parsed")])
    secondary_paths = args.secondary_corpus
    corpus = load_many(primary_paths)
    tables = SG.get_tables()
    cats, worst, scissors = category_report(args.layout, corpus, tables)
    pats = trigram_patterns(args.layout, corpus, tables)
    use = finger_usage(args.layout, corpus, tables)
    guide = fingering_guide(args.layout, corpus, tables)
    primary_score = score_layout(args.layout, corpus, tables)
    secondary_score = None
    if secondary_paths:
        secondary_score = score_layout(args.layout, load_many(secondary_paths), tables)

    lines = []
    def out(s=""):
        print(s)
        lines.append(s)
    out(f"layout: {args.layout}")
    out("grid (QWERTY order, left/right split 5+5):")
    for r in range(3):
        row = args.layout[r * 10:(r + 1) * 10]
        out(f"  {row[:5]} | {row[5:]}")
    out(f"primary={primary_score:.9f}  corpus={','.join(primary_paths)}")
    if secondary_score is not None:
        out(f"secondary={secondary_score:.9f}  corpus={','.join(secondary_paths)}")
    out()
    out("== bigram categories (% of letter bigrams, chosen fingering) ==")
    for k in ("sfb", "scissor_IM_hsb", "scissor_IM_fsb", "scissor_MR", "scissor_RP",
              "roll_in", "roll_out", "leap", "alt", "repeat"):
        out(f"  {k:16s} {cats[k]:6.2f}")
    out()
    out("== worst bigrams (chosen cost, fingering, count) ==")
    for cost, bg, f1, f2, cnt in worst[:20]:
        out(f"  {bg}  n={cnt:8d}  {cost:6.2f}  {f1}{f2}")
    out()
    out("== scissors (weight=cost*n, kind, fingering) ==")
    for w, c, bg, f1, f2, cnt, pair in scissors[:15]:
        out(f"  {bg}  n={cnt:8d}  cost={c:5.2f}  {pair:3s}  {f1}{f2}")
    out()
    out("== trigram patterns (chosen fingering, % of trigrams) ==")
    for k in ("alternate", "inroll", "outroll", "total_rolls", "onehand",
              "redirect", "redirect_weak", "total_redirects", "sfb_tri", "other"):
        out(f"  {k:16s} {pats.get(k, 0):6.2f}")
    out()
    out("== finger load (% of all strokes) ==")
    for h in "LR":
        out(f"  {h}: " + "  ".join(f"{f}={use[h + f]:5.2f}%" for f in "PRMI"))
    out()
    out("== alpha fingering guide (trigram DP) ==")
    for ch in LETTERS:
        g = guide[ch]
        shares = " ".join(f"{f}={v:.1f}%" for f, v in g["share"].items())
        out(f"  {ch}@{g['key']:<2s} default {g['default']}  {shares}")
        for f, tops in g["alt_contexts"].items():
            out("      alt %s: %s" % (f, ", ".join(f"{tg}:{n}" for tg, n in tops[:5])))

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(lines) + "\n")
    out_json.parent.mkdir(parents=True, exist_ok=True)
    out_json.write_text(json.dumps({
        "layout": args.layout,
        "primary_score": primary_score,
        "primary_corpora": primary_paths,
        "secondary_score": secondary_score,
        "secondary_corpora": secondary_paths,
        "categories": cats,
        "worst_bigrams": [[c, bg, f1, f2, n] for c, bg, f1, f2, n in worst],
        "scissors": [[w, c, bg, f1, f2, n, p] for w, c, bg, f1, f2, n, p in scissors],
        "trigram_patterns": pats,
        "usage": use,
        "guide": guide,
    }, indent=1) + "\n")
    print(f"\nwrote {out_path} and {out_json}")


if __name__ == "__main__":
    main()
