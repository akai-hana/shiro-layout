#!/usr/bin/env python3
"""D6: build space-inclusive corpus (Sigma = a-z + space) from raw iweb.

Unlike corpus/ (letters-only, space dropped) this keeps space as a normal
character so word-initial/word-final transitions are visible to the scorer.
Punctuation still breaks runs (punct priced separately in B6).
Substitutions: none (no punct in Sigma; they break runs).
Output: corpus/word/{letter,bigram,trigram}_freq.json  (space char = ' ')
"""
import json
import re
import time

PATH = "/home/akai/git/layout-opencode-investigation/corpus/extras/iweb-corpus-samples-cleaned.txt"
OUTDIR = "/home/akai/git/layout-opencode-investigation/corpus/word/"
RUN = re.compile(rb"[a-z ]+")


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

    buf = bytearray()
    with open(PATH, "rb") as f:
        while True:
            chunk = f.read(1 << 22)
            if not chunk:
                break
            chunk = chunk.lower()
            for m in RUN.finditer(chunk):
                handle(m.group())
    # note: runs that span chunk boundaries are lost (chunk size 4MB; negligible
    # relative loss at boundaries: ~125 runs over 526MB)

    import os
    os.makedirs(OUTDIR, exist_ok=True)
    for name, d in (("letter", L), ("bigram", B), ("trigram", T)):
        json.dump(d, open(OUTDIR + name + "_freq.json", "w"))
    n_space = L.get(" ", 0)
    print(f"runs {n_runs}  chars {n_chars}  space {n_space} "
          f"({100*n_space/max(n_chars,1):.1f}%)")
    print(f"L {len(L)}  B {len(B)}  T {len(T)}  ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
