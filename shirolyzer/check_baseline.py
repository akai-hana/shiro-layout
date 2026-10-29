#!/usr/bin/env python3
"""Verify reference scores, audit rulings, and the shipped-model baseline."""
import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

from utils import paths
from utils import references as R
from utils import scorer as SG
from utils.ansi import KEYS, KI

MODEL = "theta_geo_A_all.json"
BASELINE_FILE = paths.CONFIG_DIR / "score_baseline.json"
AUDIT_CONFIG = "A:all"
OLD_BASE = paths.SHIROLYZER_ROOT / "dev" / "oldbase.py"


def reference_costs(layout, corpus, tables=None):
    big, tri, _, _ = tables or SG.get_tables()
    if len(layout) != 30:
        raise SystemExit(f"{layout!r} is not a 30-character layout")
    remap = np.full(31, -1, dtype=np.int32)
    key_of = {ch: KEYS[i] for i, ch in enumerate(layout) if ch.isalpha()}
    for ci, ch in enumerate(corpus.chars):
        if ch == SG.SPACE:
            remap[ci] = SG.SPACE_ID
        elif ch in key_of:
            remap[ci] = KI[key_of[ch]]

    def cost(ids, counts, table):
        ids = remap[ids]
        ok = np.all(ids >= 0, axis=1)
        if not np.any(ok):
            return 0.0
        if ids.shape[1] == 2:
            flat = ids[ok, 0] * 31 + ids[ok, 1]
        else:
            flat = (ids[ok, 0] * 961 + ids[ok, 1] * 31 + ids[ok, 2])
        total = float(counts[ok].sum()) or 1.0
        return float((table.ravel()[flat] * counts[ok]).sum() / total)

    return {
        "bigram": cost(corpus.bi_ids, corpus.bi_counts, big),
        "trigram": cost(corpus.tri_ids, corpus.tri_counts, tri),
    }


def audit_counts():
    with tempfile.TemporaryDirectory() as tmp:
        out_json = Path(tmp) / "audit.json"
        subprocess.run(
            [sys.executable, str(paths.SHIROLYZER_ROOT / "audit.py"),
             AUDIT_CONFIG, "--out-json", str(out_json),
             "--out", str(Path(tmp) / "audit.txt")],
            cwd=paths.SHIROLYZER_ROOT, check=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        data = json.loads(out_json.read_text())
    return data["rulings"]


def oldbase_counts():
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run([sys.executable, str(OLD_BASE)], cwd=tmp, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        data = json.loads((Path(tmp) / "oldbase.json").read_text())
    return {"passed": sum(bool(v[0]) for v in data.values()), "total": len(data)}


def collect(corpus_path):
    corpus = SG.Corpus.load(corpus_path)
    tables = SG.get_tables()
    return {
        "model": MODEL,
        "audit": {"config": AUDIT_CONFIG, **audit_counts()},
        "oldbase": oldbase_counts(),
        "references": {
            entry["name"]: reference_costs(entry["layout"], corpus, tables)
            for entry in R.load_references()
        },
    }


def compare(expected, actual, tolerance):
    errors = []
    for key in ("audit", "oldbase"):
        for field in ("passed", "total"):
            if expected[key][field] != actual[key][field]:
                errors.append(f"{key}.{field}: {expected[key][field]} != {actual[key][field]}")
    if expected["model"] != actual["model"]:
        errors.append(f"model: {expected['model']} != {actual['model']}")
    expected_refs = expected.get("references", {})
    actual_refs = actual["references"]
    if set(expected_refs) != set(actual_refs):
        errors.append(f"reference names: {sorted(expected_refs)} != {sorted(actual_refs)}")
    for name in sorted(set(expected_refs) & set(actual_refs)):
        for field in ("bigram", "trigram"):
            want = float(expected_refs[name][field])
            got = float(actual_refs[name][field])
            if abs(want - got) > tolerance:
                errors.append(f"{name}.{field}: {want:.12f} != {got:.12f}")
    return errors


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--baseline", default=str(BASELINE_FILE))
    ap.add_argument("--corpus", default=str(paths.CORPUS_ROOT / "shai" / "parsed"))
    ap.add_argument("--tolerance", type=float, default=1e-9)
    ap.add_argument("--write", action="store_true",
                    help="regenerate the baseline file")
    args = ap.parse_args()
    actual = collect(args.corpus)
    baseline = Path(args.baseline)
    if args.write:
        baseline.parent.mkdir(parents=True, exist_ok=True)
        baseline.write_text(json.dumps(actual, indent=2) + "\n")
        print(f"wrote {baseline}")
        return
    if not baseline.is_file():
        raise SystemExit(f"baseline not found: {baseline}")
    expected = json.loads(baseline.read_text())
    errors = compare(expected, actual, args.tolerance)
    if errors:
        for error in errors:
            print(f"FAIL {error}")
        raise SystemExit(1)
    print("baseline OK: audit "
          f"{actual['audit']['passed']}/{actual['audit']['total']}, oldbase "
          f"{actual['oldbase']['passed']}/{actual['oldbase']['total']}, "
          f"{len(actual['references'])} references")


if __name__ == "__main__":
    main()
