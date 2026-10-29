#!/usr/bin/env python3
"""Build or verify corpora/shai from the official AKL Shai raw text.

Source: corpora/shai/iweb-corpus-samples-cleaned.txt. This is the cleaned Shai
iWeb text shared on the alt-layout community Discord.

We count maximal [a-z]+ runs after lowercasing. Space, punctuation, and every
non-ASCII byte break a run, so the letters-only corpus contains no fabricated
cross-punctuation pairs. Counts are stored as raw integers, matching the
existing corpora/*/parsed/*_freq.json convention.

Usage:
    python3 corpora/shai/parse_shai.py --check   # compare with parsed/, no writes
    python3 corpora/shai/parse_shai.py --write   # rebuild parsed/ + build_info
"""
import argparse
import hashlib
import json
import re
import time
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCE = HERE / "iweb-corpus-samples-cleaned.txt"
OUTDIR = HERE / "parsed"
RUN = re.compile(rb"[a-z]+")
CHUNK = 64 << 20


def scan(path):
    L, B, T = Counter(), Counter(), Counter()
    n_runs = 0
    md5 = hashlib.md5()

    def handle(run):
        nonlocal n_runs
        n_runs += 1
        for c in run:
            L[chr(c)] += 1
        for i in range(len(run) - 1):
            B[run[i:i + 2].decode()] += 1
        for i in range(len(run) - 2):
            T[run[i:i + 3].decode()] += 1

    t0 = time.time()
    with open(path, "rb") as f:
        tail = b""
        while True:
            chunk = f.read(CHUNK)
            if not chunk:
                break
            md5.update(chunk)
            data = (tail + chunk).lower()
            prev = None
            for m in RUN.finditer(data):
                if prev is not None:
                    handle(prev.group())
                prev = m
            tail = prev.group() if prev is not None else b""
        if tail:
            handle(tail)
    return L, B, T, n_runs, md5.hexdigest(), time.time() - t0


def load_existing(prefix):
    return tuple(
        json.load(open(prefix / f"{name}_freq.json"))
        for name in ("letter", "bigram", "trigram")
    )


def compare(name, new, old):
    keys = set(new) | set(old)
    diffs = [(k, new.get(k, 0), old.get(k, 0)) for k in keys
             if new.get(k, 0) != old.get(k, 0)]
    diffs.sort(key=lambda x: abs(x[2] - x[1]), reverse=True)
    print(f"{name}: new_sum={sum(new.values()):,} old_sum={sum(old.values()):,} "
          f"types={len(new)} diffs={len(diffs)}")
    for k, n, o in diffs[:10]:
        print(f"  {k!r}: new={n:,} old={o:,} delta={n-o:+,}")
    return diffs


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true",
                    help="overwrite parsed/ with raw-derived counts")
    ap.add_argument("--check", action="store_true",
                    help="only compare against parsed/ (default)")
    args = ap.parse_args()

    if not SOURCE.exists():
        raise SystemExit(f"missing official raw corpus: {SOURCE}")
    print(f"source: {SOURCE}")
    old = load_existing(OUTDIR) if (OUTDIR / "letter_freq.json").exists() else None
    L, B, T, n_runs, md5, elapsed = scan(SOURCE)
    print(f"scanned {sum(L.values()):,} letters in {n_runs:,} runs "
          f"in {elapsed:.0f}s; md5={md5}")
    print(f"letters={sum(L.values()):,} bigrams={sum(B.values()):,} "
          f"trigrams={sum(T.values()):,}")

    if old is not None:
        for name, new, prev in (("letters", L, old[0]), ("bigrams", B, old[1]),
                                ("trigrams", T, old[2])):
            compare(name, new, prev)

    if args.write:
        OUTDIR.mkdir(parents=True, exist_ok=True)
        for name, d in (("letter", L), ("bigram", B), ("trigram", T)):
            with open(OUTDIR / f"{name}_freq.json", "w") as f:
                json.dump(dict(sorted(d.items())), f, indent=0)
        info = "\n".join([
            "Shai iWeb letters-only corpus",
            f"source: {SOURCE}",
            f"source_md5: {md5}",
            f"letters: {sum(L.values()):,}",
            f"bigrams: {sum(B.values()):,}",
            f"trigrams: {sum(T.values()):,}",
            f"runs: {n_runs:,}",
            "processing: lowercase, maximal [a-z]+ runs; non-letters break runs",
        ]) + "\n"
        (HERE / "build_info.txt").write_text(info)
        print(f"wrote {OUTDIR}")
    else:
        print("check only; pass --write to rebuild")


if __name__ == "__main__":
    main()
