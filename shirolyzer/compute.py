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
from utils import config as CFG
from utils import diversity as DIV
from utils import paths
from utils import references as R
from utils import scorer as SG
from utils import xkb as XKB
from utils.corpus import load_many
from utils.search import LETTERS, Evaluator, dead_of, slots_of, quench, perturb


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


class ConstrainedEvaluator:
    """Secondary evaluator that rejects candidates outside the primary budget."""

    def __init__(self, primary_ev, secondary_ev, limit):
        self.primary_ev = primary_ev
        self.secondary_ev = secondary_ev
        self.limit = limit

    def score(self, perm):
        if self.primary_ev.score(perm) > self.limit:
            return math.inf
        return self.secondary_ev.score(perm)

    def layout_str(self, perm):
        return self.secondary_ev.layout_str(perm)


def polish_candidates(results, primary_ev, secondary_ev, args, rng):
    """Polish distinct near-best primary layouts until a pass stops helping."""
    rows = [r for r in results if not r["start"].startswith("secondary:")]
    ref = min(rows, key=lambda r: (r["primary"], r["final"]))
    candidates, _ = DIV.select_valid_layouts(
        rows, ref, primary_tolerance=args.valid_tolerance,
        secondary_tolerance=args.secondary_tolerance,
        min_distance=args.min_layout_distance,
        max_valid=args.polish_candidates)
    polished = []
    rounds_done = 0
    iterations_done = 0
    for i, row in enumerate(candidates, 1):
        best = list(slots_of(primary_ev, row["layout"]))
        best_s = primary_ev.score(best)
        start_s = best_s
        passes = 0
        for round_idx in range(max(1, args.polish_rounds)):
            cand = perturb(best, rng, rng.choice([4, 6, 8]))
            cand, cs = quench(primary_ev, cand, primary_ev.score(cand), rng)
            cand, cs = primary_ev.anneal(
                cand, iters=max(1, args.iters),
                seed=args.seed + 100003 + 17 * round_idx + i)
            cand, cs = quench(primary_ev, cand, cs, rng)
            rounds_done += 1
            passes += 1
            iterations_done += max(1, args.iters)
            if cs < best_s - 1e-12:
                best, best_s = cand[:], cs
            else:
                break
        layout = primary_ev.layout_str(best)
        primary = primary_ev.score(best)
        secondary = secondary_ev.score(best) if secondary_ev is not None else None
        new = {"start": f"polish:{row['start']}", "final": best_s,
               "primary": primary, "secondary": secondary, "layout": layout}
        polished.append(new)
        print(f"  polish{i} {row['start']:16s} {start_s:.4f} -> {best_s:.4f} "
              f"primary={primary:.4f} rounds={passes}  {layout}", flush=True)
    return polished, rounds_done, iterations_done


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--config", default=None,
                    help="YAML config file (default: config/config.yaml)")
    ap.add_argument("--seed-layout", "--layout", dest="seed_layout",
                    action="append", default=[],
                    help="additional starting layout (repeatable)")
    ap.add_argument("--name", default=None,
                    help="layout name used to derive default output paths")
    ap.add_argument("--primary-corpus", action="append", default=[],
                    help="parsed corpus dir, corpus dir containing parsed/, or raw .txt/.xz/.zip (repeatable)")
    ap.add_argument("--secondary-corpus", action="append", default=[],
                    help="parsed corpus dir, corpus dir containing parsed/, or raw .txt/.xz/.zip (repeatable)")
    ap.add_argument("--references", default=str(paths.REFERENCES_FILE),
                    help="TOML reference layouts (default: config/references.toml)")
    ap.add_argument("--primary-sacrifice", type=float, default=None,
                    help="percent of the best primary score allowed for secondary tuning "
                         "(default: config general.primary_sacrifice)")
    ap.add_argument("--runs", type=int, default=28,
                    help="random restarts in addition to reference seeds (default: 28)")
    ap.add_argument("--iters", type=int, default=100000,
                    help="anneal iterations per run (default: 100000)")
    ap.add_argument("--ils", type=int, default=8,
                    help="ILS rounds per top layout (default: 8)")
    ap.add_argument("--valid-tolerance", type=float, default=0.002,
                    help="primary-score window for comparable valid layouts "
                         "(default: 0.002)")
    ap.add_argument("--secondary-tolerance", type=float, default=0.005,
                    help="secondary-score window for comparable valid layouts "
                         "(default: 0.005)")
    ap.add_argument("--min-layout-distance", type=int, default=4,
                    help="minimum differing key positions between valid layouts "
                         "(default: 4)")
    ap.add_argument("--max-valid", type=int, default=8,
                    help="maximum valid layouts to report; 0 means no limit "
                         "(default: 8)")
    ap.add_argument("--polish-rounds", type=int, default=0,
                    help="maximum polish passes over distinct candidates; "
                         "0 disables polishing (default: 0)")
    ap.add_argument("--polish-candidates", type=int, default=4,
                    help="number of distinct primary candidates to polish "
                         "(default: 4)")
    ap.add_argument("--append-references", default=None,
                    help="TOML file for valid layouts (default: --references)")
    ap.add_argument("--seed", type=int,
                    help="RNG seed; omitted means generate and record one")
    ap.add_argument("--time-budget", type=parse_budget, metavar="DURATION",
                    help="wall-clock budget such as 90, 30s, 10m, or 1h "
                         "(default: no limit; stops between operations)")
    ap.add_argument("--xkb", action="store_true",
                    help="write the selected layout to xkb/<name>")
    ap.add_argument("--xkb-name", default=None,
                    help="XKB symbols file name (default: a generated safe name)")
    ap.add_argument("--out", default=None)
    ap.add_argument("--out-json", default=None)
    args = ap.parse_args()
    cfg = CFG.load_and_apply(args.config)
    out_path = Path(args.out) if args.out else paths.result_path("compute", args.name)
    out_json = (Path(args.out_json) if args.out_json
                else paths.result_path("compute", args.name, suffix=".json"))
    primary_sacrifice = (args.primary_sacrifice if args.primary_sacrifice is not None
                         else cfg.primary_sacrifice)

    started = time.time()
    completed_runs = 0
    completed_iterations = 0
    completed_ils_rounds = 0
    completed_secondary_rounds = 0
    completed_polish_rounds = 0
    seed_generated = args.seed is None
    if seed_generated:
        args.seed = random.SystemRandom().randint(0, 2**31 - 1)
    print(f"seed={args.seed}{' (generated)' if seed_generated else ''}", flush=True)
    rng = random.Random(args.seed)
    if args.primary_corpus:
        primary_paths = args.primary_corpus
        primary_corpus = load_many(primary_paths)
    else:
        primary_paths = CFG.corpus_labels(cfg, "primary")
        primary_corpus = CFG.load_corpora(cfg, "primary")
    if args.secondary_corpus:
        secondary_paths = args.secondary_corpus
        secondary_corpus = load_many(secondary_paths)
    else:
        secondary_paths = CFG.corpus_labels(cfg, "secondary")
        secondary_corpus = CFG.load_corpora(cfg, "secondary")
    tables = SG.get_tables()
    base_layout = args.seed_layout[0] if args.seed_layout else cfg.current_layout
    dead = dead_of(base_layout)
    primary_ev = Evaluator(primary_corpus, dead, tables)
    secondary_ev = Evaluator(secondary_corpus, dead, tables) if secondary_corpus else None

    seeds = []
    seen = set()
    for entry in R.load_references(args.references):
        lay = entry["layout"]
        if lay not in seen:
            seen.add(lay)
            seeds.append((entry["name"], list(slots_of(primary_ev, lay))))
    for i, lay in enumerate(args.seed_layout):
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
        if args.time_budget and results and time.time() - started >= args.time_budget:
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
        completed_runs += 1
        completed_iterations += args.iters
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
            completed_ils_rounds += 1
            completed_iterations += max(1, args.iters // 2)
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

    if args.polish_rounds > 0:
        print("\npolish (primary):", flush=True)
        polished, polish_rounds, polish_iterations = polish_candidates(
            results, primary_ev, secondary_ev, args, rng)
        completed_polish_rounds += polish_rounds
        completed_iterations += polish_iterations
        results.extend(polished)
        for row in polished:
            if row["primary"] < best_primary["primary"]:
                best_primary = row

    final = best_primary
    best_secondary = None
    if secondary_ev is not None:
        budget = abs(best_primary["primary"]) * primary_sacrifice / 100.0
        limit = best_primary["primary"] + budget + 1e-12
        constrained = ConstrainedEvaluator(primary_ev, secondary_ev, limit)

        primary_results = [r for r in results if not r["start"].startswith("secondary:")]
        primary_pool = sorted(primary_results, key=lambda r: r["primary"])[:4]
        print(f"\nsecondary phase (sacrifice {primary_sacrifice:g}% = {budget:.3g}):", flush=True)
        for r in primary_pool:
            if args.time_budget and time.time() - started >= args.time_budget:
                print("time budget reached before secondary phase", flush=True)
                break
            perm = list(slots_of(primary_ev, r["layout"]))
            cur = constrained.score(perm)
            if not math.isfinite(cur):
                continue
            best_perm, best_sec = perm[:], cur
            for it in range(max(1, args.ils)):
                if args.time_budget and time.time() - started >= args.time_budget:
                    break
                cand = perturb(best_perm, rng, rng.choice([2, 4, 6]))
                cand_s = constrained.score(cand)
                if not math.isfinite(cand_s):
                    continue
                cand, cand_s = quench(constrained, cand, cand_s, rng)
                if cand_s < best_sec - 1e-12:
                    best_perm, best_sec = cand[:], cand_s
            lay = primary_ev.layout_str(best_perm)
            pri = primary_ev.score(best_perm)
            sec = secondary_ev.score(best_perm)
            row = {"start": f"secondary:{r['start']}", "final": sec,
                   "primary": pri, "secondary": sec, "layout": lay}
            results.append(row)
            completed_secondary_rounds += 1
            if best_secondary is None or sec < best_secondary["secondary"]:
                best_secondary = row
            print(f"  {r['start']:16s} primary={pri:.6f} secondary={sec:.6f}  {lay}", flush=True)
        if best_secondary is not None:
            final = best_secondary
        budget = abs(best_primary["primary"]) * primary_sacrifice / 100.0
        limit = best_primary["primary"] + budget + 1e-12
        eligible = [r for r in results
                    if r.get("secondary") is not None and r["primary"] <= limit]
        if eligible:
            best_secondary = min(eligible, key=lambda r: r["secondary"])
            final = best_secondary

    valid_rows, valid_truncated = DIV.select_valid_layouts(
        results, best_primary, primary_tolerance=args.valid_tolerance,
        secondary_tolerance=args.secondary_tolerance,
        min_distance=args.min_layout_distance, max_valid=args.max_valid)
    if not any(row["layout"] == final["layout"] for row in valid_rows):
        merged = next((row for row in DIV.merge_results(results)
                       if row["layout"] == final["layout"]), None)
        valid_rows.append(merged or final)
    if not valid_rows:
        valid_rows = [final]
    valid_rows.sort(key=lambda row: (
        row["layout"] != final["layout"], row["primary"],
        row["secondary"] if row["secondary"] is not None else math.inf))
    selection_status = ("clear_winner" if len(valid_rows) == 1
                        else "multiple_valid")
    final = valid_rows[0]

    append_path = (Path(args.append_references) if args.append_references
                   else Path(args.references))
    existing_names = {entry["layout"]: entry["name"]
                      for entry in R.load_references(append_path)}
    base_name = args.name or f"shiro-{args.seed}"
    added_names = []
    for i, row in enumerate(valid_rows):
        requested = base_name if i == 0 else f"{base_name}-{i + 1}"
        added = R.append_reference(
            requested, row["layout"], primary_score=row["primary"],
            secondary_score=row.get("secondary"), seed=args.seed,
            path=append_path)
        if added:
            existing_names[row["layout"]] = added
            added_names.append(added)
            print(f"appended reference {added!r} to {append_path}")
        row["reference_name"] = existing_names.get(row["layout"])
    added_name = added_names[0] if added_names else None

    xkb_path = None
    if args.xkb:
        xkb_name = args.xkb_name or XKB.safe_name(final["layout"])
        xkb_path = paths.XKB_DIR / xkb_name
        XKB.write(xkb_path, final["layout"], xkb_name)
        print(f"wrote {xkb_path}")

    results.sort(key=lambda r: (r["primary"], r["final"]))
    out_lines = [
        f"seed={args.seed} generated={str(seed_generated).lower()}",
        f"primary={','.join(primary_paths)} secondary={','.join(secondary_paths)}",
        f"references={args.references}",
        f"append_references={append_path}",
        f"reference_added={added_name or ''}",
        f"references_added={','.join(added_names)}",
        f"primary_sacrifice_pct={primary_sacrifice:g}",
        f"selection={selection_status} "
        f"valid_layouts_truncated={str(valid_truncated).lower()}",
        (f"completed_runs={completed_runs} completed_iterations={completed_iterations} "
         f"completed_ils_rounds={completed_ils_rounds} "
         f"completed_secondary_rounds={completed_secondary_rounds}"),
        f"best_primary: primary={best_primary['primary']:.9f} {best_primary['layout']}",
        (f"best_secondary: primary={best_secondary['primary']:.9f} "
         f"secondary={best_secondary['secondary']:.9f} {best_secondary['layout']}"
         if best_secondary is not None else "best_secondary: none"),
        f"selected: primary={final['primary']:.9f} "
        f"secondary={final['secondary'] if final['secondary'] is not None else float('nan'):.9f} "
        f"{final['layout']}",
        *DIV.format_valid_layouts(valid_rows, selection_status, valid_truncated),
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
        "seed_generated": seed_generated,
        "primary_corpora": primary_paths,
        "secondary_corpora": secondary_paths,
        "references": args.references,
        "primary_sacrifice_pct": primary_sacrifice,
        "elapsed_seconds": time.time() - started,
        "time_budget_seconds": args.time_budget,
        "completed_runs": completed_runs,
        "completed_iterations": completed_iterations,
        "completed_ils_rounds": completed_ils_rounds,
        "completed_secondary_rounds": completed_secondary_rounds,
        "completed_polish_rounds": completed_polish_rounds,
        "runs": args.runs, "iters": args.iters, "ils": args.ils,
        "polish_rounds": args.polish_rounds,
        "polish_candidates": args.polish_candidates,
        "valid_tolerance": args.valid_tolerance,
        "secondary_tolerance": args.secondary_tolerance,
        "min_layout_distance": args.min_layout_distance,
        "max_valid": args.max_valid,
        "results": results,
        "best_primary": best_primary,
        "best_secondary": best_secondary,
        "selected": final,
        "selection_status": selection_status,
        "valid_layouts": DIV.valid_payload(valid_rows),
        "valid_layouts_truncated": valid_truncated,
        "reference_added": added_name,
        "references_added": added_names,
        "append_references": str(append_path),
        "xkb": str(xkb_path) if xkb_path is not None else None,
    }, indent=1) + "\n")
    print(f"\nbest_primary: {best_primary['layout']} ({best_primary['primary']:.6f})")
    print(f"selected: {final['layout']} ({selection_status})")
    for line in DIV.format_valid_layouts(valid_rows, selection_status,
                                         valid_truncated):
        print(line)
    print(f"wrote {out_path} and {out_json} ({time.time() - started:.0f}s)")


if __name__ == "__main__":
    main()
