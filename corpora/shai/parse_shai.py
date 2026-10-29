#!/usr/bin/env python3
"""Build or verify corpora/shai from the official AKL Shai raw text.

Source: https://colemak.com/pub/corpus/iweb-corpus-samples-cleaned.txt.xz
This is the cleaned Shai iWeb text shared on the alt-layout community Discord.

The source is downloaded to a temporary file, decompressed as a stream, and
deleted on success. We count maximal [a-z]+ runs after lowercasing. Space,
punctuation, and every non-ASCII byte break a run, so the letters-only corpus
contains no fabricated cross-punctuation pairs. Counts are stored as raw
integers, matching the existing corpora/*/parsed/*_freq.json convention.

Usage:
    python3 corpora/shai/parse_shai.py            # download, parse, write parsed/
    python3 corpora/shai/parse_shai.py --check    # download, parse, compare only
    python3 corpora/shai/parse_shai.py --source FILE [--keep-source]
"""
import argparse
import hashlib
import json
import lzma
import os
import re
import tempfile
import time
import urllib.request
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUTDIR = HERE / "parsed"
DEFAULT_URL = "https://colemak.com/pub/corpus/iweb-corpus-samples-cleaned.txt.xz"
RUN = re.compile(rb"[a-z]+")
CHUNK = 64 << 20
DOWNLOAD_CHUNK = 1 << 20
EXPECTED = {
    "letters": 418_381_828,
    "bigrams": 327_253_737,
    "trigrams": 240_524_895,
}


def download(url, dest):
    print(f"downloading {url}")
    request = urllib.request.Request(url, headers={"User-Agent": "shirolyzer-corpus/1.0"})
    with urllib.request.urlopen(request, timeout=120) as response, open(dest, "wb") as out:
        total = int(response.headers.get("Content-Length") or 0)
        done = last = 0
        while True:
            chunk = response.read(DOWNLOAD_CHUNK)
            if not chunk:
                break
            out.write(chunk)
            done += len(chunk)
            if done - last >= 32 << 20:
                if total:
                    print(f"  downloaded {done / 1e6:.1f} / {total / 1e6:.1f} MB")
                else:
                    print(f"  downloaded {done / 1e6:.1f} MB")
                last = done
    print(f"  downloaded {done / 1e6:.1f} MB total")


def open_source(path):
    with open(path, "rb") as probe:
        magic = probe.read(6)
    if magic == b"\xfd7zXZ\x00":
        return lzma.open(path, "rb")
    return open(path, "rb")


def scan(stream):
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
    tail = b""
    while True:
        chunk = stream.read(CHUNK)
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


def write_json(path, data):
    with open(path, "w") as f:
        json.dump(dict(sorted(data.items())), f, indent=0)
        f.write("\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--source", type=Path,
                    help="existing .txt or .txt.xz source instead of downloading")
    ap.add_argument("--url", default=DEFAULT_URL,
                    help=f"override the download URL (default: {DEFAULT_URL})")
    ap.add_argument("--check", action="store_true",
                    help="compare against parsed/ without writing (default: write)")
    ap.add_argument("--write", action="store_true",
                    help="write parsed/ (default; kept for compatibility)")
    ap.add_argument("--keep-source", action="store_true",
                    help="keep a downloaded temporary source for debugging")
    args = ap.parse_args()
    if args.check and args.write:
        ap.error("--check and --write are mutually exclusive")

    temp_path = None
    if args.source:
        source = args.source
        if not source.exists():
            raise SystemExit(f"missing source: {source}")
        source_label = str(source)
    else:
        fd, temp_name = tempfile.mkstemp(prefix="shai-iweb-", suffix=".txt.xz")
        os.close(fd)
        temp_path = Path(temp_name)
        download(args.url, temp_path)
        source = temp_path
        source_label = args.url

    print(f"source: {source_label}")
    old = load_existing(OUTDIR) if (OUTDIR / "letter_freq.json").exists() else None
    try:
        with open_source(source) as stream:
            L, B, T, n_runs, md5, elapsed = scan(stream)
    finally:
        if temp_path is not None:
            if args.keep_source:
                print(f"kept downloaded source: {temp_path}")
            else:
                temp_path.unlink(missing_ok=True)

    totals = {
        "letters": sum(L.values()),
        "bigrams": sum(B.values()),
        "trigrams": sum(T.values()),
    }
    print(f"scanned {totals['letters']:,} letters in {n_runs:,} runs "
          f"in {elapsed:.0f}s; md5={md5}")
    print(f"letters={totals['letters']:,} bigrams={totals['bigrams']:,} "
          f"trigrams={totals['trigrams']:,}")
    for name, value in totals.items():
        expected = EXPECTED[name]
        print(f"  {name}: expected={expected:,} "
              f"{'OK' if value == expected else 'MISMATCH'}")

    all_diffs = []
    if old is not None:
        for name, new, prev in (("letters", L, old[0]), ("bigrams", B, old[1]),
                                ("trigrams", T, old[2])):
            all_diffs.extend(compare(name, new, prev))

    if args.check:
        if all_diffs:
            raise SystemExit("check failed: parsed data differs")
        print("check OK: parsed data matches")
        return

    OUTDIR.mkdir(parents=True, exist_ok=True)
    for name, data in (("letter", L), ("bigram", B), ("trigram", T)):
        write_json(OUTDIR / f"{name}_freq.json", data)
    print(f"wrote {OUTDIR}")


if __name__ == "__main__":
    main()
