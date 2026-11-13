#!/usr/bin/env python3
"""Run config variants across seeds and report preference metrics.

Development tooling only. The Rust binary is the runtime implementation.

A variants file is either a JSON/YAML list or an object with a ``variants``
list.  Each entry has a ``name`` and an optional ``overrides`` mapping.  Keys in
``overrides`` may be dotted paths, for example::

    [
      {"name": "baseline", "overrides": {}},
      {"name": "redirect-5", "overrides": {"redirect_scale": 5.0}},
      {"name": "steeper", "overrides": {
          "redirect_tiers": [["1-2-1"], ["3-2-3"], ["1-3-2"]]
      }}
    ]

Each seed runs ``generate`` with the variant config, then ``analyze`` on the
selected layout with both configured corpora.  The output JSON contains the
generate rows and the full analyze preference block for every variant and seed.
"""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import yaml

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
        help="base config file",
    )
    parser.add_argument(
        "--variants",
        default=None,
        help="JSON/YAML variants file; omit to sweep the base config only",
    )
    parser.add_argument("--start-seed", type=int, default=0, help="first seed")
    parser.add_argument("--seeds", type=int, default=1, help="number of seeds")
    parser.add_argument("--runs", type=int, default=28, help="generate --runs")
    parser.add_argument("--iters", type=int, default=200_000, help="generate --iters")
    parser.add_argument("--ils", type=int, default=8, help="generate --ils")
    parser.add_argument(
        "--ils-candidates", type=int, default=4, help="generate --ils-candidates"
    )
    parser.add_argument(
        "--batch-size", type=int, default=8, help="generate --batch-size"
    )
    parser.add_argument(
        "--secondary-rounds", type=int, default=None, help="generate --secondary-rounds"
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
        "--out-json",
        default=None,
        help="output path (default: layouts/preference_sweep_<timestamp>.json)",
    )
    return parser


def load_variants(path: str | None) -> list[dict]:
    if path is None:
        return [{"name": "baseline", "overrides": {}}]
    text = Path(path).read_text()
    if path.endswith(".json"):
        data = json.loads(text)
    else:
        data = yaml.safe_load(text)
    if isinstance(data, dict):
        data = data.get("variants", [data])
    if not isinstance(data, list) or not data:
        raise SystemExit("variants file must contain a non-empty list")
    out = []
    for index, item in enumerate(data):
        if not isinstance(item, dict):
            raise SystemExit(f"variant {index} must be a mapping")
        name = item.get("name", f"variant-{index + 1}")
        overrides = item.get("overrides", {})
        if not isinstance(overrides, dict):
            raise SystemExit(f"variant {name!r} overrides must be a mapping")
        out.append({"name": str(name), "overrides": overrides})
    return out


def set_dotted(target: dict, path: str, value) -> None:
    parts = path.split(".")
    node = target
    for part in parts[:-1]:
        child = node.get(part)
        if not isinstance(child, dict):
            child = {}
            node[part] = child
        node = child
    node[parts[-1]] = value


def write_variant_config(base: dict, overrides: dict, path: Path) -> None:
    value = copy.deepcopy(base)
    for key, override in overrides.items():
        set_dotted(value, key, override)
    path.write_text(yaml.safe_dump(value, sort_keys=False))


def run_json(command: list[str], output: Path) -> dict:
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(
            f"command failed: {' '.join(command)}\n{result.stderr.strip()}"
        )
    return json.loads(output.read_text())


def generate_command(
    args: argparse.Namespace,
    config: Path,
    seed: int,
    references: Path,
    out_txt: Path,
    out_json: Path,
) -> list[str]:
    command = [
        args.binary,
        "generate",
        "--config",
        str(config),
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
        "--append-references",
        str(references),
        "--out",
        str(out_txt),
        "--out-json",
        str(out_json),
    ]
    if args.secondary_rounds is not None:
        command += ["--secondary-rounds", str(args.secondary_rounds)]
    if args.polish_rounds is not None:
        command += ["--polish-rounds", str(args.polish_rounds)]
    if args.time_budget is not None:
        command += ["--time-budget", args.time_budget]
    return command


def analyze_command(
    args: argparse.Namespace,
    config: Path,
    layout: str,
    out_txt: Path,
    out_json: Path,
) -> list[str]:
    return [
        args.binary,
        "analyze",
        layout,
        "--config",
        str(config),
        "--out",
        str(out_txt),
        "--out-json",
        str(out_json),
    ]


def main() -> int:
    args = build_parser().parse_args()
    binary = Path(args.binary)
    if not binary.is_file():
        raise SystemExit(f"binary not found: {binary}; run `just build` first")
    base_path = Path(args.config)
    base = yaml.safe_load(base_path.read_text())
    if not isinstance(base, dict):
        raise SystemExit(f"{base_path}: config must be a YAML mapping")
    variants = load_variants(args.variants)

    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out_path = (
        Path(args.out_json)
        if args.out_json
        else ROOT / "layouts" / f"preference_sweep_{timestamp}.json"
    )

    records = []
    summaries = []
    with tempfile.TemporaryDirectory(prefix="shirolyzer-sweep-") as tmp_name:
        tmp = Path(tmp_name)
        for variant in variants:
            config_path = tmp / f"{variant['name']}.yaml"
            write_variant_config(base, variant["overrides"], config_path)
            variant_records = []
            for seed in range(args.start_seed, args.start_seed + args.seeds):
                stem = f"{variant['name']}-{seed}"
                references = tmp / f"{stem}-refs.toml"
                references.write_text("# generated by preference_sweep.py\n")
                out_txt = tmp / f"{stem}-generate.txt"
                out_json = tmp / f"{stem}-generate.json"
                generate = run_json(
                    generate_command(
                        args,
                        config_path,
                        seed,
                        references,
                        out_txt,
                        out_json,
                    ),
                    out_json,
                )
                layout = (
                    generate.get("selected", {}).get("layout")
                    or generate["best_primary"]["layout"]
                )
                analysis_txt = tmp / f"{stem}-analysis.txt"
                analysis_json = tmp / f"{stem}-analysis.json"
                analysis = run_json(
                    analyze_command(
                        args,
                        config_path,
                        layout,
                        analysis_txt,
                        analysis_json,
                    ),
                    analysis_json,
                )
                record = {
                    "variant": variant["name"],
                    "seed": seed,
                    "layout": layout,
                    "generate": {
                        "best_primary": generate.get("best_primary"),
                        "selected": generate.get("selected"),
                    },
                    "analysis": {
                        "primary_score": analysis.get("primary_score"),
                        "secondary_score": analysis.get("secondary_score"),
                        "preference": analysis.get("preference"),
                    },
                }
                records.append(record)
                variant_records.append(record)

            summaries.append(summarize(variant["name"], variant_records))

    payload = {
        "generated_at": timestamp,
        "base_config": str(base_path),
        "variants_file": args.variants,
        "seeds": {
            "start": args.start_seed,
            "count": args.seeds,
        },
        "generate": {
            "runs": args.runs,
            "iters": args.iters,
            "ils": args.ils,
            "ils_candidates": args.ils_candidates,
            "batch_size": args.batch_size,
            "secondary_rounds": args.secondary_rounds,
            "polish_rounds": args.polish_rounds,
            "time_budget": args.time_budget,
        },
        "summary": summaries,
        "records": records,
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(payload, indent=2) + "\n")

    print(f"wrote {out_path}")
    for summary in summaries:
        print(
            f"{summary['variant']}: "
            f"primary={summary['mean_primary_score']:.9f} "
            f"secondary={summary['mean_secondary_score']:.9f} "
            f"redirects={summary['mean_redirect_share']:.3f}% "
            f"max_scissor_band_cost={summary['mean_max_scissor_band_cost']:.6f}"
        )
    return 0


def mean(values: list[float]) -> float:
    return sum(values) / len(values) if values else 0.0


def summarize(name: str, records: list[dict]) -> dict:
    primaries = [
        record["analysis"]["primary_score"]
        for record in records
        if record["analysis"].get("primary_score") is not None
    ]
    secondaries = [
        record["analysis"]["secondary_score"]
        for record in records
        if record["analysis"].get("secondary_score") is not None
    ]
    redirects = []
    max_scissor_costs = []
    roll_pairs = {}
    for record in records:
        primary = (record["analysis"].get("preference") or {}).get("primary")
        if not primary:
            continue
        redirect = primary["redirects"]
        total = primary["total_trigrams"]
        redirects.append(redirect["count"] / total * 100.0 if total else 0.0)
        max_scissor_costs.append(
            max(band["weighted_cost"] for band in primary["scissor_bands"])
        )
        for key, metric in primary["rolls"].items():
            roll_pairs.setdefault(key, []).append(metric["share"])
    return {
        "variant": name,
        "runs": len(records),
        "mean_primary_score": mean(primaries),
        "mean_secondary_score": mean(secondaries),
        "mean_redirect_share": mean(redirects),
        "mean_max_scissor_band_cost": mean(max_scissor_costs),
        "mean_roll_shares": {
            key: mean(values) for key, values in sorted(roll_pairs.items())
        },
    }


if __name__ == "__main__":
    sys.exit(main())
