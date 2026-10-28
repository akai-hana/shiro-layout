#!/usr/bin/env python3
"""D2 corpus: Sigma = a-z + {',', '.', ';', '/', "'", '-'} over raw iweb.

Extends B6's punct corpus with apostrophe and hyphen (both now normal
in-run characters, no longer run-breakers). Substitutions: '?'->'/', ':'->';'.
Output: corpus/punct2/{letter,bigram,trigram}_freq.json
"""
import json
import os
import re
import time

PATH = "/home/akai/git/layout-opencode-investigation/corpus/extras/iweb-corpus-samples-cleaned.txt"
OUTDIR = "/home/akai/git/layout-opencode-investigation/corpus/punct2/"
SIGMA = set("abcdefghijklmnopqrstuvwxyz,./;'-")
SUB_TBL = bytes(
    ord("/") if b == ord("?") else ord(";") if b == ord(":") else b
    for b in range(256))
RUN = re.compile(rb"[a-z,./;'-]+")


def main():
    L, B, T = {}, {}, {}
    t0 = time.time()
    n_runs = n_chars = 0

    def handle(run):
        nonlocal n_runs, n_chars
        n_runs += 1
        n_chars += len(run)
        for c in run:
            ch = chr(c)
            L[ch] = L.get(ch, 0) + 1
        for i in range(len(run) - 1):
            k = run[i:i + 2].decode()
            B[k] = B.get(k, 0) + 1
        for i in range(len(run) - 2):
            k = run[i:i + 3].decode()
            T[k] = T.get(k, 0) + 1

    with open(PATH, "rb") as f:
        while True:
            chunk = f.read(1 << 22)
            if not chunk:
                break
            chunk = chunk.translate(SUB_TBL).lower()
            for m in RUN.finditer(chunk):
                handle(m.group())

    os.makedirs(OUTDIR, exist_ok=True)
    for name, d in (("letter", L), ("bigram", B), ("trigram", T)):
        json.dump(d, open(OUTDIR + name + "_freq.json", "w"))
    mass = {ch: L.get(ch, 0) for ch in ".,/;'-"}
    tot = sum(L.values())
    print(f"runs {n_runs}  chars {n_chars}  ({time.time()-t0:.0f}s)")
    print("punct mass:", {k: f"{100*v/tot:.2f}%" for k, v in mass.items()})
    print(f"L {len(L)}  B {len(B)}  T {len(T)}")


if __name__ == "__main__":
    main()
