#!/usr/bin/env python3
"""D3: rank the A2 field under keycraft's published weight files.

score = sum_w (weight_class * keysolve_rate), keycraft convention
(better = higher). Uses weights.txt and weights2.txt from keycraft's
repo. Load-deviation terms (HLD/FLD/RLD/POH) omitted — not in the
keysolve metric report; they are secondary (|w| <= 4 on ~0.01-0.1 rates).
"""
import json
import re

WFILES = {
    "keycraft weights.txt": "/tmp/opencode/analyzers/keycraft/data/config/weights.txt",
    "keycraft weights2.txt": "/tmp/opencode/analyzers/keycraft/data/config/weights2.txt",
}
REPORT = ("/home/akai/git/layout-opencode-investigation/results/"
          "keysolve_report.json")
OUT = ("/home/akai/git/layout-opencode-investigation/results/"
       "d3_published_weights.txt")


def parse_weights(path):
    w = {}
    for line in open(path):
        line = line.split("#")[0].strip()
        if not line or "=" not in line:
            continue
        k, v = line.split("=", 1)
        try:
            w[k.strip()] = float(v)
        except ValueError:
            pass
    return w


def score_from_weights(m, w):
    """Map keycraft weight keys onto keysolve metrics (rates)."""
    s = 0.0
    mapping = {
        "SFB": "SFB", "LSB": "LSB", "FSB": "FSB", "HSB": "HSB",
        "SFS": "SFS", "LSS": "LSS", "FSS": "FSS", "HSS": "HSS",
        "FLW": "ROL",   # flow ~ roll share
        "RED-WEAK": "RED",  # coarse: whole RED gets the weak weight
        "RED": "RED",
    }
    used_red = False
    for wk, mk in mapping.items():
        if wk not in w:
            continue
        if wk in ("RED", "RED-WEAK"):
            if used_red:
                continue
            used_red = True
        s += w[wk] * m.get(mk, 0.0)
    # ALT / 2RL / 3RL / RED-SFS / RED-NML / IN:OUT have w ~ 0 -> skip
    return s


def main():
    data = json.load(open(REPORT))
    lines = ["D3 — field ranking under keycraft published weights "
             "(keysolve rates; load-dev terms omitted)", ""]
    all_rows = []
    for wname, wpath in WFILES.items():
        w = parse_weights(wpath)
        rows = []
        for name, m in data.items():
            rows.append((score_from_weights(m, w), name))
        rows.sort(reverse=True)  # higher = better
        lines.append(f"== {wname} ==")
        for i, (s, name) in enumerate(rows, 1):
            lines.append(f"  {i:2d}. {s:+8.4f}  {name}")
        lines.append("")
        all_rows.append((wname, rows))

    # agreement with our objective
    ours = ["cfgE_semiQ (ours)", "cfgG_semiP (alt)", "Sturdy", "Graphite",
            "Colemak-DH", "Canary", "Semimak", "Gallium", "Colemak", "QWERTY"]
    lines.append("(cfgE rank under each system: " +
                 ", ".join(f"{wname} -> "
                           f"{[n for _, n in rows].index('cfgE_semiQ (ours)')+1}"
                           for wname, rows in all_rows) + ")")
    txt = "\n".join(lines) + "\n"
    open(OUT, "w").write(txt)
    print(txt)


if __name__ == "__main__":
    main()
