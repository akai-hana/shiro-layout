#!/usr/bin/env python3
"""Measure run-to-run spread of `shirolyzer generate` across seeds.

Development tooling only. The Rust binary is the runtime implementation.
"""

from __future__ import annotations

import argparse
import json
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--binary",
        default=str(ROOT / "target/release/shirolyzer"),
        help="path to the release binary",
    )
    parser.add_argument(
        "--config",
        default=str(ROOT / "config/config.yaml"),
        help="config file passed to generate",
    )
    parser.add_argument("--start-seed", type=int, default=0, help="first seed")
    parser.add_argument("--seeds", type=int, default=20, help="number of seeds")
    parser.add_argument("--runs", type=int, default=28, help="generate --runs")
    parser.add_argument("--iters", type=int, default=200_000, help="generate --iters")
    parser.add_argument("--ils", type=int, default=8, help="generate --ils")
    parser.add_argument(
        "--ils-candidates", type=int, default=4, help="generate --ils-candidates"
    )
    parser.add_argument(
        "--batch-size", type=int, default=8, help="generate --batch-size"
    )
    parser.add_argument("--stop-hits", type=int, default=0, help="generate --stop-hits")
    parser.add_argument(
        "--stop-stale-batches",
        type=int,
        default=0,
        help="generate --stop-stale-batches",
    )
    parser.add_argument(
        "--polish-rounds",
        default=None,
        help="generate --polish-rounds (omit for the binary default)",
    )
    parser.add_argument(
        "--time-budget", default=None, help="generate --time-budget"
    )
    parser.add_argument(
        "--tolerance",
        type=float,
        default=0.002,
        help="primary tolerance used to classify a seed as a success",
    )
    parser.add_argument(
        "--verbose", action="store_true", help="print one line per seed"
    )
    return parser


def run_seed(args: argparse.Namespace, seed: int, append_path: Path) -> dict:
    with tempfile.TemporaryDirectory(prefix="shirolyzer-variance-") as tmp:
        out_json = Path(tmp) / "generate.json"
        out_txt = Path(tmp) / "generate.txt"
        command = [
            args.binary,
            "generate",
            "--config",
            args.config,
            "--seed",
            str(seed),
            "--runs",
            str(args.runs),
            "--iters",
            str(args.iters),
            "--ils",
            str(args.ils),
            "--ils-candidates",
            str(args.ils_candidates),
            "--batch-size",
            str(args.batch_size),
            "--stop-hits",
            str(args.stop_hits),
            "--stop-stale-batches",
            str(args.stop_stale_batches),
            "--append-references",
            str(append_path),
            "--out",
            str(out_txt),
            "--out-json",
            str(out_json),
        ]
        if args.polish_rounds is not None:
            command += ["--polish-rounds", str(args.polish_rounds)]
        if args.time_budget is not None:
            command += ["--time-budget", args.time_budget]
        result = subprocess.run(command, capture_output=True, text=True)
        if result.returncode != 0:
            raise SystemExit(
                f"generate failed for seed {seed}:\n{result.stderr.strip()}"
            )
        return json.loads(out_json.read_text())


def main() -> int:
    args = build_parser().parse_args()
    binary = Path(args.binary)
    if not binary.is_file():
        raise SystemExit(f"binary not found: {binary}; run `just build` first")

    seeds = list(range(args.start_seed, args.start_seed + args.seeds))
    cases = []
    with tempfile.TemporaryDirectory(prefix="shirolyzer-variance-refs-") as tmp:
        append_path = Path(tmp) / "references.toml"
        append_path.write_text("")
        for seed in seeds:
            print(f"seed {seed} ...", file=sys.stderr, flush=True)
            value = run_seed(args, seed, append_path)
            best = value["best_primary"]
            cases.append(
                {
                    "seed": seed,
                    "primary": float(best["primary"]),
                    "layout": best["layout"],
                    "stop_reason": value["stop_reason"],
                    "completed_runs": value["completed_runs"],
                    "completed_batches": value["completed_batches"],
                }
            )

    values = [case["primary"] for case in cases]
    best = min(values)
    worst = max(values)
    mean = statistics.fmean(values)
    stdev = statistics.pstdev(values) if len(values) > 1 else 0.0
    successes = sum(1 for value in values if value - best <= args.tolerance)
    success_rate = successes / len(values)

    if args.verbose:
        for case in cases:
            distance = case["primary"] - best
            print(
                f"seed {case['seed']:>3}  primary={case['primary']:.9f}  "
                f"delta={distance:.9f}  runs={case['completed_runs']}  "
                f"batches={case['completed_batches']}  "
                f"stop={case['stop_reason']}  {case['layout']}"
            )

    print(
        f"seeds={len(seeds)} range={seeds[0]}..{seeds[-1]} "
        f"runs={args.runs} iters={args.iters} ils={args.ils} "
        f"ils_candidates={args.ils_candidates} batch_size={args.batch_size}"
    )
    print(
        f"best={best:.9f} min={best:.9f} max={worst:.9f} mean={mean:.9f} "
        f"stdev={stdev:.9f} spread={worst - best:.9f}"
    )
    print(
        f"success_rate={success_rate:.1%} ({successes}/{len(values)} seeds "
        f"within {args.tolerance:.9f} of best)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
