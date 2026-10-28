#!/usr/bin/env python3
"""A2: distance-resolved same-finger skipgram ladder on raw text.

Scans corpus/extras/iweb-corpus-samples-cleaned.txt over genkey ValidChars runs
(SkipgramsMustSpanValidChars style), computes per-gap same-finger rates
SFS_1..SFS_4 (gap = letters skipped between the pair), repeats a==b excluded
(keycraft convention), standard columnar fingering, cfge punct positions
('?',''->dead keys, '_'/'-' unmapped).

Also: all-gap SFS% (keysolve convention) and SFB with genkey/keysolve
denominators as a cross-check against findings/analyzer_comparison.md 6.1.
"""
import re
import sys
import time

PATH = "/home/akai/git/layout-opencode-investigation/corpus/extras/iweb-corpus-samples-cleaned.txt"
OUT = "/home/akai/git/layout-opencode-investigation/results/sfs_ladder.txt"

CFGE = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
FINGER = ([0, 1, 2, 3, 3, 6, 6, 7, 8, 9]) * 3
LETTERS = "abcdefghijklmnopqrstuvwxyz"
VALID_CHARS = "abcdefghijklmnopqrstuvwxyz,./?;:'\"_-"
# genkey CharSubstitutions: ? -> / , : -> ; , _ -> - , " -> '
SUB = {"?": "/", ":": ";", "_": "-", '"': "'"}
PUNCT_ON_GRID = {"'", ",", ".", "/", ";"}  # on cfge 30-key grid

run_re = re.compile(rb"[a-z,./?;:'\"_\-]+")
MAXGAP = 4
INV = 255


def build_table():
    grid = {c: i for i, c in enumerate(CFGE)}
    tbl = bytearray([INV] * 256)
    for c in VALID_CHARS:
        c2 = SUB.get(c, c)
        if c2 in grid:
            tbl[ord(c)] = FINGER[grid[c2]]
        # valid but not on this layout (e.g. -) stays INV (denominator only)
    return bytes(tbl)


def main():
    tbl = build_table()
    sf = {g: [0, 0] for g in range(MAXGAP + 1)}   # gap -> [same, total]
    allgap = [0, 0]
    bigram = [0, 0]           # [same_finger, pairs]
    valid_chars = 0           # all chars in valid runs (incl unmapped)
    t0 = time.time()

    def handle(run):
        nonlocal valid_chars
        n = len(run)
        if n == 0:
            return
        valid_chars += n
        fs = run.translate(tbl)
        for i in range(n - 1):
            bigram[1] += 1
            a, b = fs[i], fs[i + 1]
            if a != INV and a == b and run[i] != run[i + 1]:
                bigram[0] += 1
        for i in range(n):
            for j in range(i + 2, n):
                gap = min(j - i - 1, MAXGAP)
                sf[gap][1] += 1
                allgap[1] += 1
                a, b = fs[i], fs[j]
                if a != INV and a == b and run[i] != run[j]:
                    sf[gap][0] += 1
                    allgap[0] += 1

    with open(PATH, "rb") as f:
        tail = b""
        while True:
            chunk = f.read(32 << 20)
            if not chunk:
                break
            data = tail + chunk.lower()
            # cut at last whitespace so runs don't span chunks
            last = 0
            for ws in (0x20, 0x0a, 0x0d, 0x09):
                p = data.rfind(ws)
                if p > last:
                    last = p
            cut = last + 1
            head, tail = data[:cut], data[cut:]
            for run in run_re.finditer(head):
                handle(run.group())
        # tail
        for run in run_re.finditer(tail):
            handle(run.group())

    # letters totals over whole file (genkey-style denominators)
    # recount cheaply: we only have head-chunks; recount in a second pass
    letters_all = 0
    letters_plus_space = 0
    letters_plus_space_punct = 0
    with open(PATH, "rb") as f:
        while True:
            chunk = f.read(64 << 20)
            if not chunk:
                break
            d = chunk.lower()
            letters_all += sum(1 for c in d if 97 <= c <= 122)
            letters_plus_space += sum(
                1 for c in d if (97 <= c <= 122) or c == 0x20)
            letters_plus_space_punct += sum(
                1 for c in d if (97 <= c <= 122) or c in (0x20, 0x2c, 0x2e,
                0x2f, 0x3b, 0x27, 0x2d, 0x3a, 0x3f, 0x22, 0x5f))

    r = 1.0 / 7.0
    lines = [
        "A2 — SFS distance ladder (raw iweb samples, cfge layout fingers,",
        "standard columnar map, repeats a==b excluded, valid-char runs)",
        f"scanned {valid_chars:,} run-chars in {time.time()-t0:.0f}s",
        "",
        f"{'gap':>4s} {'same-finger':>14s} {'pairs':>14s} {'SFS%':>8s}",
    ]
    weighted = wsum = 0.0
    for g in range(1, MAXGAP + 1):
        same, tot = sf[g]
        pct = 100 * same / tot
        w = r ** (g - 1)
        weighted += pct * w
        wsum += w
        lines.append(f"{g:>4d} {same:>14,} {tot:>14,} {pct:8.3f}")
    lines.append("(gap 4+ folded into 4)")
    lines.append("")
    lines.append(f"all-gap SFS%  : {100*allgap[0]/allgap[1]:.3f}   "
                 f"[{allgap[0]:,}/{allgap[1]:,}]")
    lines.append(f"oxey decay ladder (r=1/7 normalized over gaps 1-4): "
                 f"{weighted/wsum:.3f}%")
    lines.append("")
    lines.append("SFB cross-check (cfge punct positions):")
    lines.append(f"  bigram denom (pairs in valid runs) : "
                 f"{100*bigram[0]/bigram[1]:.3f}%   "
                 f"[{bigram[0]:,}/{bigram[1]:,}]  (doc keycraft-style 4.48)")
    lines.append(f"  monogram denom, letters only       : "
                 f"{100*bigram[0]/letters_all:.3f}%")
    lines.append(f"  monogram denom, letters+space      : "
                 f"{100*bigram[0]/letters_plus_space:.3f}%  "
                 f"(closest to genkey Total; doc genkey-style 3.624)")
    lines.append(f"  monogram denom, letters+space+punct: "
                 f"{100*bigram[0]/letters_plus_space_punct:.3f}%")
    txt = "\n".join(lines) + "\n"
    print(txt)
    with open(OUT, "w") as f:
        f.write(txt)
    print("wrote", OUT)


if __name__ == "__main__":
    main()
