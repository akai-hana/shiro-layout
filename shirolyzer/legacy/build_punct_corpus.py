#!/usr/bin/env python3
"""B6: build punct-aware corpus over Sigma = a-z + {',', '.', ';', '/'}.

Raw source: corpus/extras/iweb-corpus-samples-cleaned.txt (same file behind
genkey's shai-iweb). Processing:
  - lowercase
  - genkey CharSubstitutions: '?'->'/', ':'->';'  (same physical key, shifted)
  - runs = maximal Sigma-char sequences; space/other chars break runs
    (so "word, word" contributes 'd,' + ','+' ' boundary — the space pair
    itself is dropped, consistent with the letters-only convention)
  - monograms/bigrams/trigrams counted over runs
Apostrophe is NOT in Sigma (our layout keeps it on ISO-102, off the 30-key
grid); runs break there so no cross-apostrophe pairs are fabricated.
Output: corpus/punct/{letter,bigram,trigram}_freq.json
"""
import json
import re
import time

PATH = "/home/akai/git/layout-opencode-investigation/corpus/extras/iweb-corpus-samples-cleaned.txt"
OUTDIR = "/home/akai/git/layout-opencode-investigation/corpus/punct/"
SIGMA = set("abcdefghijklmnopqrstuvwxyz,./;")
SUB_TBL = bytes(
    ord("/") if b == ord("?") else ord(";") if b == ord(":") else b
    for b in range(256))
RUN = re.compile(rb"[a-z,./;]+")


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
        n = len(run)
        for i in range(n - 1):
            bg = run[i:i + 2].decode()
            B[bg] = B.get(bg, 0) + 1
        for i in range(n - 2):
            tg = run[i:i + 3].decode()
            T[tg] = T.get(tg, 0) + 1

    with open(PATH, "rb") as f:
        tail = b""
        while True:
            chunk = f.read(32 << 20)
            if not chunk:
                break
            data = (tail + chunk).lower().translate(SUB_TBL)
            last = 0
            for ws in (0x20, 0x0a, 0x0d, 0x09):
                p = data.rfind(ws)
                if p > last:
                    last = p
            cut = last + 1
            head, tail = data[:cut], data[cut:]
            for m in RUN.finditer(head):
                handle(m.group())
        for m in RUN.finditer(tail):
            handle(m.group())

    import os
    os.makedirs(OUTDIR, exist_ok=True)
    json.dump(L, open(OUTDIR + "letter_freq.json", "w"))
    json.dump(B, open(OUTDIR + "bigram_freq.json", "w"))
    json.dump(T, open(OUTDIR + "trigram_freq.json", "w"))
    punct_total = sum(v for k, v in L.items() if k not in "abcdefghijklmnopqrstuvwxyz")
    lines = [
        f"punct corpus: {n_chars:,} chars in {n_runs:,} runs "
        f"({time.time()-t0:.0f}s)",
        f"letters {sum(v for k,v in L.items() if k.isalpha()):,}  "
        f"punct {punct_total:,} ({100*punct_total/n_chars:.2f}% of Sigma mass)",
        f"bigram types {len(B)}  trigram types {len(T)}",
        "top punct chars: " + ", ".join(
            f"{k} {v:,}" for k, v in sorted(
                ((k, v) for k, v in L.items() if k not in "abcdefghijklmnopqrstuvwxyz"),
                key=lambda x: -x[1])),
        "top punct bigrams: " + ", ".join(
            f"{k} {v:,}" for k, v in sorted(
                ((k, v) for k, v in B.items()
                 if not (k[0].isalpha() and k[1].isalpha())),
                key=lambda x: -x[1])[:12]),
    ]
    txt = "\n".join(lines) + "\n"
    print(txt)
    with open(OUTDIR + "build_info.txt", "w") as f:
        f.write(txt)


if __name__ == "__main__":
    main()
