#!/usr/bin/env python3
"""Anneal under the calibrated Shai-primary objective.

Default: English Shai is primary; Spanish is a near-tie secondary.  The
`--primary-sacrifice` value is the maximum primary-score regression that the
final selector treats as a tie, and also the composite weight (one full
secondary-score unit is worth that many primary-score units).  Set it to 0
for a pure-Shai run.
"""
import argparse
import json
import random
import sys
import time

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from compare import dead_of, slots_of
from objective import (DEFAULT_PRIMARY_SACRIFICE, PRIMARY_PATH,
                       SECONDARY_PATH, build_evaluators_full, load_primary,
                       load_secondary, within_primary_budget)
from ils import quench, perturb

CFG_E = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
SHAI_BEST = ";tspbxgoiwndhrlyecuafkvmzjq,./"
DEFAULT_SEEDS = [
    ("shai_best", SHAI_BEST),
    ("chat_best", ";thskxpoiyrdgnwfeucalzmbvqj,./"),
    ("old_shai_lead", ";trdpzgaifsmcnlxehuowkvbjqy,./"),
    ("old_shai_alt1", ";tncbxgaifsdprlkehuowzvmjqy,./"),
    ("old_shai_alt2", ";thsbxcoiwndprlyeguafkvmzqj,./"),
    ("cfgE", CFG_E),
]
N = 26


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--primary", default=PRIMARY_PATH)
    ap.add_argument("--secondary", default=SECONDARY_PATH)
    ap.add_argument("--primary-sacrifice", type=float,
                    default=DEFAULT_PRIMARY_SACRIFICE)
    ap.add_argument("--runs", type=int, default=24)
    ap.add_argument("--iters", type=int, default=60000)
    ap.add_argument("--ils", type=int, default=6)
    ap.add_argument("--seed", type=int, default=917)
    ap.add_argument("--seed-layout", action="append", default=[])
    ap.add_argument("--out", default="/home/akai/git/layout-opencode-investigation/results/round8/anneal_shai.txt")
    ap.add_argument("--out-json", default="/home/akai/git/layout-opencode-investigation/results/round8/anneal_shai.json")
    args = ap.parse_args()
    if args.primary_sacrifice < 0:
        ap.error("--primary-sacrifice must be >= 0")
    return args


def main():
    args = parse_args()
    primary_corpus = load_primary(args.primary)
    secondary_corpus = load_secondary(args.secondary)
    ev, pev, sev, scale = build_evaluators_full(
        dead_of(CFG_E), args.primary_sacrifice,
        primary=primary_corpus, secondary=secondary_corpus)
    rng = random.Random(args.seed)
    t0 = time.time()
    print(f"primary={args.primary} secondary={args.secondary} "
          f"sacrifice={args.primary_sacrifice:g} scale={scale:.6g}", flush=True)

    seeds = [("cfgE", CFG_E)]
    seeds += DEFAULT_SEEDS
    for i, lay in enumerate(args.seed_layout):
        seeds.append((f"user{i}", lay))
    starts = []
    seen = set()
    for name, lay in seeds:
        if len(lay) != 30:
            print(f"skip {name}: layout length {len(lay)}", flush=True)
            continue
        if lay in seen:
            continue
        seen.add(lay)
        starts.append((name, list(slots_of(ev, lay))))
    for s in range(args.runs):
        perm = list(range(N))
        random.Random(args.seed + 1000 + s).shuffle(perm)
        starts.append((f"rand{args.seed + 1000 + s}", perm))

    results = []
    print(f"{'start':16s} {'quench':>8s} {'anneal':>8s} {'final':>8s} "
          f"{'primary':>8s} {'spanish':>8s}  layout", flush=True)
    for name, start in starts:
        ts = time.time()
        cur, cs = quench(ev, start[:], ev.score(tuple(start)),
                         random.Random(7))
        aperm, ans = ev.anneal(seed_perm=cur, iters=args.iters, seed=17)
        aperm, q2 = quench(ev, aperm, ans, random.Random(27))
        lay = ev.layout_str(tuple(aperm))
        slots = tuple(slots_of(ev, lay))
        pri = pev.score(slots)
        sec = sev.score(slots)
        results.append({"start": name, "quench": cs, "anneal": ans,
                        "final": q2, "primary": pri, "spanish": sec,
                        "layout": lay})
        print(f"{name:16s} {cs:8.4f} {ans:8.4f} {q2:8.4f} "
              f"{pri:8.4f} {sec:8.4f}  {lay}  ({time.time()-ts:.0f}s)",
              flush=True)

    uniq = {}
    for r in results:
        lay = r["layout"]
        if lay not in uniq or r["final"] < uniq[lay]["final"]:
            uniq[lay] = r
    top = sorted(uniq.values(), key=lambda r: r["final"])[:4]
    print("\nILS:", flush=True)
    for rank, r in enumerate(top, 1):
        lay = r["layout"]
        best = list(slots_of(ev, lay))
        best_s = r["final"]
        for it in range(args.ils):
            cand = perturb(best, rng, rng.choice([4, 6, 8]))
            cand, cs = quench(ev, cand, ev.score(tuple(cand)), rng)
            cand, cs = ev.anneal(seed_perm=cand, iters=max(1, args.iters // 2),
                                 seed=51 + it)
            cand, cs = quench(ev, cand, cs, rng)
            if cs < best_s - 1e-12:
                best, best_s = cand[:], cs
        ilay = ev.layout_str(tuple(best))
        islots = tuple(slots_of(ev, ilay))
        ipri, isec = pev.score(islots), sev.score(islots)
        print(f"  ILS{rank} {r['start']:16s} {r['final']:.4f} -> "
              f"{best_s:.4f} primary={ipri:.4f} spanish={isec:.4f}  {ilay}",
              flush=True)
        results.append({"start": f"ILS{rank}:{r['start']}", "quench": r["final"],
                        "anneal": best_s, "final": best_s, "primary": ipri,
                        "spanish": isec, "layout": ilay})

    results.sort(key=lambda r: r["final"])
    best_primary = min(results, key=lambda r: (r["primary"], r["final"]))
    near = [r for r in results
            if within_primary_budget(r["primary"], best_primary["primary"],
                                     args.primary_sacrifice)]
    best_near = min(near, key=lambda r: (r["spanish"], r["primary"],
                                         r["final"]))
    best_comp = min(results, key=lambda r: r["final"])

    lines = [
        f"primary={args.primary} secondary={args.secondary}",
        f"primary_sacrifice={args.primary_sacrifice:g} secondary_scale={scale:.6g}",
        f"best_primary: comp={best_primary['final']:.6f} "
        f"primary={best_primary['primary']:.6f} "
        f"spanish={best_primary['spanish']:.6f} {best_primary['layout']}",
        f"best_near_tie: comp={best_near['final']:.6f} "
        f"primary={best_near['primary']:.6f} "
        f"spanish={best_near['spanish']:.6f} {best_near['layout']}",
        f"best_composite: comp={best_comp['final']:.6f} "
        f"primary={best_comp['primary']:.6f} "
        f"spanish={best_comp['spanish']:.6f} {best_comp['layout']}",
        "",
        f"{'start':20s} {'final':>8s} {'primary':>8s} {'spanish':>8s}  layout",
    ]
    for r in results:
        lines.append(f"{r['start']:20s} {r['final']:8.4f} "
                     f"{r['primary']:8.4f} {r['spanish']:8.4f}  {r['layout']}")
    txt = "\n".join(lines) + "\n"
    open(args.out, "w").write(txt)
    json.dump({"primary": args.primary, "secondary": args.secondary,
               "primary_sacrifice": args.primary_sacrifice,
               "secondary_scale": scale, "results": results,
               "best_primary": best_primary, "best_near_tie": best_near,
               "best_composite": best_comp},
              open(args.out_json, "w"), indent=1)
    print(f"\nbest_primary: {best_primary['layout']} "
          f"(primary={best_primary['primary']:.6f})")
    print(f"best_near_tie: {best_near['layout']} "
          f"(primary={best_near['primary']:.6f} "
          f"spanish={best_near['spanish']:.6f})")
    print(f"best_composite: {best_comp['layout']} "
          f"(comp={best_comp['final']:.6f})")
    print(f"wrote {args.out} and {args.out_json} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
