#!/usr/bin/env python3
"""Build Spanish letter/bigram/trigram frequency files from raw texts.

Input : directory of .txt files (UTF-8). Provenance (2026-01-20):
  - Project Gutenberg Spanish books: 2000 (Don Quijote), 17013
    (Fortunata y Jacinta), 17073 (La Regenta), 13264 (Zalacaín),
    13519 (La Tierra de Todos), 14995 (Los Hombres de Pro), 9980
    (Platero y yo) — ~8 MB, 19th/early-20th century literary Spanish.
  - es.wikipedia.org full extracts of 26 major articles — ~1 MB,
    contemporary register.

Normalization: lowercase; Unicode NFKD then strip combining marks, so
á→a, é→e, ü→u and ñ→n (NFKD decomposes ñ to n + tilde). Only [a-z]
kept; everything else is a word boundary. Bigrams/trigrams are
within-word only (same convention as the English corpora).

Usage: python3 corpora/spanish/parse_spanish.py [input_dir] [output_dir]
       Defaults: input_dir=corpora/spanish/resources, output_dir=corpora/spanish/parsed
"""
import json
import sys
import unicodedata
from pathlib import Path


def normalize(text: str) -> str:
    text = unicodedata.normalize("NFKD", text.lower())
    text = "".join(ch for ch in text if not unicodedata.combining(ch))
    out = []
    for ch in text:
        out.append(ch if "a" <= ch <= "z" else " ")
    return "".join(out)


HERE = Path(__file__).resolve().parent


def main() -> None:
    in_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else HERE / "resources"
    out_prefix = sys.argv[2] if len(sys.argv) > 2 else HERE / "parsed"
    letters: dict[str, int] = {}
    bigrams: dict[str, int] = {}
    trigrams: dict[str, int] = {}
    n_letters = n_spaces = 0
    files = sorted(in_dir.glob("*.txt"))
    if not files:
        sys.exit(f"no .txt files in {in_dir}")
    for path in files:
        text = normalize(path.read_text(encoding="utf-8", errors="ignore"))
        words = text.split()
        n_letters += sum(len(w) for w in words)
        n_spaces += len(words) - 1
        for w in words:
            for ch in w:
                letters[ch] = letters.get(ch, 0) + 1
            for i in range(len(w) - 1):
                bg = w[i : i + 2]
                bigrams[bg] = bigrams.get(bg, 0) + 1
            for i in range(len(w) - 2):
                tg = w[i : i + 3]
                trigrams[tg] = trigrams.get(tg, 0) + 1
        print(f"  {path.name}: cumulative letters={n_letters:,}")

    out_dir = Path(out_prefix)
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, data in (
        ("letter_freq.json", letters),
        ("bigram_freq.json", bigrams),
        ("trigram_freq.json", trigrams),
    ):
        with open(out_dir / name, "w") as f:
            json.dump(data, f)
    space_frac = n_spaces / (n_letters + n_spaces)
    top = sorted(letters.items(), key=lambda kv: -kv[1])[:12]
    print(f"letters={n_letters:,} spaces={n_spaces:,} space_frac={space_frac:.4f}")
    print("top letters:", " ".join(f"{c}:{n/n_letters*100:.1f}%" for c, n in top))
    print(f"wrote {out_dir}/{{letter,bigram,trigram}}_freq.json")


if __name__ == "__main__":
    main()
