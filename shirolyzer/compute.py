#!/usr/bin/env python3
"""Generate a layout with the calibrated pattern model.

The primary corpora drive the quench and anneal loops.  Secondary corpora are
optimized only after primary convergence, inside the primary-sacrifice budget.
"""
import argparse
import json
import math
import random
import time
from pathlib import Path

from utils import comfort as cg
from utils import paths
from utils import scorer as SG
from utils.search import LETTERS, Evaluator, dead_of, slots_of, quench, perturb
DEFAULT_LAYOUT = ";tspbxgoiwndhrlyecuafkvmzjq,./"
DEFAULT_SEEDS = (
    ("shai_best", DEFAULT_LAYOUT),
    ("cfgE", ";aiojvlnrwhfpeygtcmsbxquzkd,./"),
    ("chat_best", ";thskxpoiyrdgnwfeucalzmbvqj,./"),
    ("qwerty", "qwertyuiopasdfghjkl;zxcvbnm,./"),
    ("colemak", "qwfpgjluy;arstdhneiozxcvbkm,./"),
    ("colemak_dh", "qwfpbjluy;arstgmneiozxcdvkh,./"),
    ("canary", "wlypkzxou;crstbfneiajvdgqmh/,. "),
    ("sturdy", "vmlcpxfoujsrtdy.naeizkqgwbh';,"),
    ("gallium", "bldcvjfou,nrtsgyhaeixqmwzkp';."),
    ("semimak", "flhvzqwuoysrntkcdeaix'bmjpg/,/"),
)


def parse_budget(value):
    if value is None:
        return None
    value = str(value).strip().lower()
    try:
        if value.endswith("ms"):
            return float(value[:-2]) / 1000
        if value.endswith("s"):
            return float(value[:-1])
        if value.endswith("m"):
            return float(value[:-1]) * 60
        if value.endswith("h"):
            return float(value[:-1]) * 3600
        return float(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"invalid time budget: {value}")


class BudgetExceeded(Exception):
    pass


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--layout", action="append", default=[],
                    help="additional starting layout (repeatable)")
    ap.add_argument("--name", default=None,
                    help="layout name used to derive default output paths")
    ap.add_argument("--primary-corpus", action="append", default=[])
    ap.add_argument("--secondary-corpus", action="append", default=[])
    ap.add_argument("--primary-sacrifice", type=float, default=0.00023,
                    help="percent of the best primary score allowed for secondary tuning")
    ap.add_argument("--runs", type=int, default=24)
    ap.add_argument("--iters", type=int, default=60000)
    ap.add_argument("--ils", type=int, default=6)
    ap.add_argument("--seed", type=int)
    ap.add_argument("--time-budget", type=parse_budget)
    ap.add_argument("--out", default=None)
    ap.add_argument("--out-json", default=None)
    args = ap.parse_args()
    out_path = Path(args.out) if args.out else paths.result_path("compute", args.name)
    out_json = (Path(args.out_json) if args.out_json
                else paths.result_path("compute", args.name, suffix=".json"))

    started = time.time()
    if args.seed is None:
        args.seed = random.SystemRandom().randint(0, 2**31 - 1)
    print(f"seed={args.seed}", flush=True)
    rng = random.Random(args.seed)
    primary_paths = args.primary_corpus or [str(paths.CORPUS_ROOT / "shai" / "parsed")]
    secondary_paths = (args.secondary_corpus
                       or [str(paths.CORPUS_ROOT / "spanish" / "parsed")])
    primary_corpora = [SG.Corpus.load(p) for p in primary_paths]
    primary_corpus = primary_corpora[0]
    if len(primary_corpora) > 1:
        from collections import Counter
        L, B, T = Counter(), Counter(), Counter()
        for c in primary_corpora:
            L.update(c.L); B.update(c.B); T.update(c.T)
        primary_corpus = SG.Corpus(L, B, T)
    secondary_corpus = SG.Corpus.load(secondary_paths[0]) if secondary_paths else None
    tables = SG.get_tables()
    base_layout = args.layout[0] if args.layout else DEFAULT_LAYOUT
    dead = dead_of(base_layout)
    primary_ev = Evaluator(primary_corpus, dead, tables)
    secondary_ev = Evaluator(secondary_corpus, dead, tables) if secondary_corpus else None

    seeds = []
    seen = set()
    for name, lay in DEFAULT_SEEDS:
        if len(lay) == 30 and lay not in seen:
            seen.add(lay)
            seeds.append((name, list(slots_of(primary_ev, lay))))
    for i, lay in enumerate(args.layout):
        if len(lay) == 30 and lay not in seen:
            seen.add(lay)
            seeds.append((f"user{i}", list(slots_of(primary_ev, lay))))
    this_run = time.time()
    for s in range(args.runs):
        perm = list(range(26))
        random.Random(args.seed + 1000 + s).shuffle(perm)
        seeds.append((f"rand{args.seed + 1000 + s}", perm))

    results = []
    best_primary = None
    print(f"{'start':18s} {'final':>9s} {'primary':>9s}  layout", flush=True)
    for name, start in seeds:
        if args.time_budget and time.time() - started >= args.time_budget:
            print("time budget reached; saving best so far", flush=True)
            break
        perm = start[:]
        cs = primary_ev.score(perm)
        try:
            perm, cs = quench(primary_ev, perm, cs, rng)
            perm, cs = primary_ev.anneal(perm, iters=args.iters, seed=args.seed + 17)
            perm, cs = quench(primary_ev, perm, cs, rng)
        except Exception:
            raise
        lay = primary_ev.layout_str(perm)
        pri = primary_ev.score(perm)
        results.append({"start": name, "final": cs, "primary": pri,
                        "secondary": None, "layout": lay})
        if best_primary is None or pri < best_primary["primary"]:
            best_primary = results[-1]
        print(f"{name:18s} {cs:9.4f} {pri:9.4f}  {lay}", flush=True)

    if not results:
        raise SystemExit("no completed runs")

    uniq = {}
    for r in results:
        if r["layout"] not in uniq or r["final"] < uniq[r["layout"]]["final"]:
            uniq[r["layout"]] = r
    top = sorted(uniq.values(), key=lambda r: (r["primary"], r["final"]))[:4]
    print("\nILS (primary):", flush=True)
    for rank, r in enumerate(top, 1):
        if args.time_budget and time.time() - started >= args.time_budget:
            print("time budget reached during ILS; saving best so far", flush=True)
            break
        best = list(slots_of(primary_ev, r["layout"]))
        best_s = r["final"]
        for it in range(max(1, args.ils)):
            if args.time_budget and time.time() - started >= args.time_budget:
                break
            cand = perturb(best, rng, rng.choice([4, 6, 8]))
            cand, cs = quench(primary_ev, cand, primary_ev.score(cand), rng)
            cand, cs = primary_ev.anneal(cand, iters=max(1, args.iters // 2), seed=51 + it)
            cand, cs = quench(primary_ev, cand, cs, rng)
            if cs < best_s - 1e-12:
                best, best_s = cand[:], cs
        ilay = primary_ev.layout_str(best)
        ipri = primary_ev.score(best)
        results.append({"start": f"ILS{rank}:{r['start']}", "final": best_s,
                        "primary": ipri, "secondary": None, "layout": ilay})
        if ipri < best_primary["primary"]:
            best_primary = results[-1]
        print(f"  ILS{rank} {r['start']:16s} {r['final']:.4f} -> {best_s:.4f} "
              f"primary={ipri:.4f}  {ilay}", flush=True)

    final = best_primary
    if secondary_ev is not None:
        budget = abs(best_primary["primary"]) * args.primary_sacrifice / 100.0
        limit = best_primary["primary"] + budget + 1e-12

        def constrained(perm):
            pri = primary_ev.score(perm)
            if pri > limit:
                return float("inf")
            return secondary_ev.score(perm)

        candidates = sorted(uniq.values(), key=lambda r: (r["primary"], r["final"]))[:4]
        print(f"\nsecondary phase (sacrifice {args.primary_sacrifice:g}% = {budget:.3g}):", flush=True)
        best_secondary = None
        for r in candidates:
            if args.time_budget and time.time() - started >= args.time_budget:
                print("time budget reached before secondary phase", flush=True)
                break
            perm = list(slots_of(primary_ev, r["layout"]))
            cur = constrained(perm)
            if not math.isfinite(cur):
                continue
            for it in range(max(1, args.ils)):
                if args.time_budget and time.time() - started >= args.time_budget:
                    break
                cand = perturb(perm, rng, rng.choice([2, 4]))
                c = constrained(cand)
                if math.isfinite(c) and c < cur - 1e-12:
                    perm, cur = cand, c
            lay = primary_ev.layout_str(perm)
            pri = primary_ev.score(perm)
            sec = secondary_ev.score(perm)
            row = {"start": f"secondary:{r['start']}", "final": sec,
                   "primary": pri, "secondary": sec, "layout": lay}
            results.append(row)
            if best_secondary is None or sec < best_secondary["secondary"]:
                best_secondary = row
            print(f"  {r['start']:16s} primary={pri:.6f} secondary={sec:.6f}  {lay}", flush=True)
        if best_secondary is not None:
            final = best_secondary

    results.sort(key=lambda r: (r["primary"], r["final"]))
    out_lines = [
        f"seed={args.seed}",
        f"primary={','.join(primary_paths)} secondary={','.join(secondary_paths)}",
        f"primary_sacrifice_pct={args.primary_sacrifice:g}",
        f"best_primary: primary={best_primary['primary']:.9f} {best_primary['layout']}",
        f"selected: primary={final['primary']:.9f} "
        f"secondary={final['secondary'] if final['secondary'] is not None else float('nan'):.9f} "
        f"{final['layout']}",
        "",
        f"{'start':24s} {'primary':>10s} {'secondary':>10s}  layout",
    ]
    for r in results:
        sec = r["secondary"]
        out_lines.append(f"{r['start']:24s} {r['primary']:10.6f} "
                         f"{(sec if sec is not None else float('nan')):10.6f}  {r['layout']}")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(out_lines) + "\n")
    out_json.parent.mkdir(parents=True, exist_ok=True)
    out_json.write_text(json.dumps({
        "seed": args.seed,
        "primary_corpora": primary_paths,
        "secondary_corpora": secondary_paths,
        "primary_sacrifice_pct": args.primary_sacrifice,
        "elapsed_seconds": time.time() - started,
        "runs": args.runs, "iters": args.iters, "ils": args.ils,
        "results": results,
        "best_primary": best_primary,
        "selected": final,
    }, indent=1) + "\n")
    print(f"\nbest_primary: {best_primary['layout']} ({best_primary['primary']:.6f})")
    print(f"selected: {final['layout']}")
    print(f"wrote {out_path} and {out_json} ({time.time() - started:.0f}s)")


if __name__ == "__main__":
    main()
