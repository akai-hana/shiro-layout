#!/usr/bin/env python3
"""Build Spanish letter/bigram/trigram frequency files from the Zenodo corpus.

Source: https://zenodo.org/records/5501931
Download: https://zenodo.org/records/5501931/files/Spanish-Keyboard-Corpus-1.0.0.zip?download=1

Only ``clean-spanish-book-extracts.txt`` is extracted from the archive.  It is
the cleaned Spanish book corpus that matches the scale of the previous local
Gutenberg sample.  The archive is downloaded to a temporary location, the
selected text file is extracted to a temporary directory, and both are deleted
on success.

Normalization: lowercase; Unicode NFKD then strip combining marks, so á→a,
é→e, ü→u and ñ→n (NFKD decomposes ñ to n + tilde). Only [a-z] is kept;
everything else is a word boundary. Bigrams/trigrams are within-word only
(same convention as the English corpora). Counts are stored as raw integers,
matching corpora/shai/parsed/*_freq.json.

Usage:
    python3 corpora/spanish/parse_spanish.py            # download, parse, write parsed/
    python3 corpora/spanish/parse_spanish.py --check    # download, parse, compare only
    python3 corpora/spanish/parse_spanish.py --source FILE.zip [--keep-source]
"""
import argparse
import hashlib
import json
import shutil
import tempfile
import unicodedata
import urllib.request
import zipfile
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUTDIR = HERE / "parsed"
DEFAULT_URL = (
    "https://zenodo.org/records/5501931/files/"
    "Spanish-Keyboard-Corpus-1.0.0.zip?download=1"
)
SOURCE_MEMBERS = ("clean-spanish-book-extracts.txt",)
DOWNLOAD_CHUNK = 1 << 20
EXPECTED = {
    "letters": 8_584_068,
    "bigrams": 6_723_441,
    "trigrams": 4_983_706,
}


def normalize(text):
    text = unicodedata.normalize("NFKD", text.lower())
    text = "".join(ch for ch in text if not unicodedata.combining(ch))
    return "".join(ch if "a" <= ch <= "z" else " " for ch in text)


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


def extract_members(zip_path, dest, members):
    with zipfile.ZipFile(zip_path) as archive:
        names = set(archive.namelist())
        missing = [name for name in members if name not in names]
        if missing:
            raise SystemExit(f"missing members in {zip_path}: {', '.join(missing)}")
        dest.mkdir(parents=True, exist_ok=True)
        for name in members:
            target = dest / Path(name).name
            with archive.open(name) as src, open(target, "wb") as out:
                shutil.copyfileobj(src, out, DOWNLOAD_CHUNK)
            print(f"  extracted {name}")


def scan_files(paths):
    letters, bigrams, trigrams = Counter(), Counter(), Counter()
    n_letters = n_spaces = 0
    md5 = hashlib.md5()
    for path in paths:
        raw = path.read_bytes()
        md5.update(raw)
        text = normalize(raw.decode("utf-8", errors="ignore"))
        words = text.split()
        n_letters += sum(len(word) for word in words)
        n_spaces += len(words) - 1
        for word in words:
            for ch in word:
                letters[ch] += 1
            for i in range(len(word) - 1):
                bigrams[word[i:i + 2]] += 1
            for i in range(len(word) - 2):
                trigrams[word[i:i + 3]] += 1
        print(f"  {path.name}: cumulative letters={n_letters:,}")
    return letters, bigrams, trigrams, n_letters, n_spaces, md5.hexdigest()


def load_existing():
    return tuple(
        json.load(open(OUTDIR / f"{name}_freq.json"))
        for name in ("letter", "bigram", "trigram")
    )


def compare(name, new, old):
    keys = set(new) | set(old)
    diffs = [(k, new.get(k, 0), old.get(k, 0)) for k in keys
             if new.get(k, 0) != old.get(k, 0)]
    diffs.sort(key=lambda item: abs(item[2] - item[1]), reverse=True)
    print(f"{name}: new_sum={sum(new.values()):,} old_sum={sum(old.values()):,} "
          f"types={len(new)} diffs={len(diffs)}")
    for key, n, o in diffs[:10]:
        print(f"  {key!r}: new={n:,} old={o:,} delta={n - o:+,}")
    return diffs


def write_json(path, data):
    with open(path, "w") as f:
        json.dump(dict(sorted(data.items())), f, indent=0)
        f.write("\n")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--source", type=Path,
                        help="existing .zip source instead of downloading")
    parser.add_argument("--url", default=DEFAULT_URL,
                        help=f"override the download URL (default: {DEFAULT_URL})")
    parser.add_argument("--check", action="store_true",
                        help="compare against parsed/ without writing (default: write)")
    parser.add_argument("--write", action="store_true",
                        help="write parsed/ (default; kept for compatibility)")
    parser.add_argument("--keep-source", action="store_true",
                        help="keep the downloaded archive and extracted text for debugging")
    args = parser.parse_args()
    if args.check and args.write:
        parser.error("--check and --write are mutually exclusive")

    work_dir = Path(tempfile.mkdtemp(prefix="spanish-corpus-"))
    temp_zip = None
    try:
        if args.source:
            source = args.source
            if not source.exists():
                raise SystemExit(f"missing source: {source}")
            source_label = str(source)
        else:
            temp_zip = work_dir / "source.zip"
            download(args.url, temp_zip)
            source = temp_zip
            source_label = args.url

        print(f"source: {source_label}")
        extract_dir = work_dir / "extracted"
        extract_members(source, extract_dir, SOURCE_MEMBERS)
        paths = sorted(extract_dir.glob("*.txt"))
        if not paths:
            raise SystemExit(f"no .txt files extracted in {extract_dir}")

        old = load_existing() if (OUTDIR / "letter_freq.json").exists() else None
        letters, bigrams, trigrams, n_letters, n_spaces, md5 = scan_files(paths)
        totals = {
            "letters": sum(letters.values()),
            "bigrams": sum(bigrams.values()),
            "trigrams": sum(trigrams.values()),
        }
        print(f"scanned {totals['letters']:,} letters, {n_spaces:,} spaces; md5={md5}")
        print(f"letters={totals['letters']:,} bigrams={totals['bigrams']:,} "
              f"trigrams={totals['trigrams']:,}")
        for name, value in totals.items():
            expected = EXPECTED[name]
            print(f"  {name}: expected={expected:,} "
                  f"{'OK' if value == expected else 'MISMATCH'}")

        all_diffs = []
        if old is not None:
            for name, new, previous in (
                ("letters", letters, old[0]),
                ("bigrams", bigrams, old[1]),
                ("trigrams", trigrams, old[2]),
            ):
                all_diffs.extend(compare(name, new, previous))

        if args.check:
            if all_diffs:
                raise SystemExit("check failed: parsed data differs")
            print("check OK: parsed data matches")
            return

        OUTDIR.mkdir(parents=True, exist_ok=True)
        for name, data in (("letter", letters), ("bigram", bigrams), ("trigram", trigrams)):
            write_json(OUTDIR / f"{name}_freq.json", data)
        print(f"wrote {OUTDIR}")
    finally:
        if args.keep_source:
            print(f"kept working files: {work_dir}")
        else:
            shutil.rmtree(work_dir, ignore_errors=True)


if __name__ == "__main__":
    main()
