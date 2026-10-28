#!/usr/bin/env python3
"""Build the primary English chat+writing corpus and a secondary code corpus."""
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def load(prefix):
    return tuple(
        json.load(open(f"{prefix}{name}_freq.json"))
        for name in ("letter", "bigram", "trigram")
    )

def normalize(counter, total=1_000_000.0):
    s = sum(counter.values()) or 1.0
    scale = total / s
    return {k: v * scale for k, v in counter.items()}

def add(dst, src, weight):
    for k, v in src.items():
        dst[k] += v * weight

def write(prefix, L, B, T):
    out = ROOT / prefix
    out.mkdir(parents=True, exist_ok=True)
    for name, d in (("letter", L), ("bigram", B), ("trigram", T)):
        clean = {("".join(k) if isinstance(k, tuple) else k): v for k, v in d.items()}
        json.dump(clean, open(out / f"{name}_freq.json", "w"), indent=0)

def blend(sources):
    L, B, T = Counter(), Counter(), Counter()
    for prefix, weight in sources:
        l, b, t = load(prefix)
        add(L, normalize(l), weight)
        add(B, normalize(b), weight)
        add(T, normalize(t), weight)
    return L, B, T

def code_corpus():
    L, B, T = Counter(), Counter(), Counter()
    files = sorted(ROOT.glob("analysis/*.py")) + [ROOT / "xkb" / "install.sh"]
    for path in files:
        text = path.read_text(errors="ignore").lower()
        for run in re.findall(r"[a-z]+", text):
            L.update(run)
            B.update(zip(run, run[1:]))
            T.update(zip(run, run[1:], run[2:]))
    return L, B, T

def main():
    L, B, T = blend([
        (str(ROOT / "corpus") + "/", 0.50),
        (str(ROOT / "corpus") + "/opensubtitles_50k_", 0.50),
    ])
    write("corpus/chat_write", L, B, T)
    L, B, T = code_corpus()
    write("corpus/code", L, B, T)
    print("chat_write trigrams:", len(T), "code trigrams:", len(T))

if __name__ == "__main__":
    main()
