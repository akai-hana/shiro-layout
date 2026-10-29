#!/usr/bin/env python3
"""Score reference layouts against a corpus with the calibrated model."""
import argparse

from utils import paths
from utils import scorer as SG
from utils.ansi import KEYS, KI

REFERENCES = {
    "qwerty": "qwertyuiopasdfghjkl;zxcvbnm,./",
    "colemak": "qwfpgjluy;arstdhneiozxcvbkm,./",
    "colemak-dh": "qwfpbjluy;arstgmneiozxcdvkh,./",
    "canary": "wlypkzxou;crstbfneiajvdgqmh/,.",
    "sturdy": "vmlcpxfoujsrtdy.naeizkqgwbh';,",
    "gallium": "bldcvjfou,nrtsgyhaeixqmwzkp';.",
    "semimak": "flhvzqwuoysrntkcdeai x'bmjpg/,/".replace(" ", ""),
    "shiro": ";tspbxgoiwndhrlyecuafkvmzjq,./",
}


def score(layout, corpus, tables=None):
    big, tri, _, _ = tables or SG.get_tables()
    if len(layout) != 30:
        raise SystemExit(f"{layout!r} is not a 30-character layout")
    m = {ch: KEYS[i] for i, ch in enumerate(layout) if ch.isalpha()}
    import numpy as np
    remap = np.full(31, -1, dtype=np.int32)
    for ci, ch in enumerate(corpus.chars):
        if ch == SG.SPACE:
            remap[ci] = SG.SPACE_ID
        elif ch in m:
            remap[ci] = KI[m[ch]]
    tri_ids = remap[corpus.tri_ids]
    ok = np.all(tri_ids >= 0, axis=1)
    flat = tri_ids[ok, 0] * 961 + tri_ids[ok, 1] * 31 + tri_ids[ok, 2]
    total = float(corpus.tri_counts[ok].sum()) or 1.0
    return float((tri.ravel()[flat] * corpus.tri_counts[ok]).sum() / total)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--primary-corpus", default=str(paths.CORPUS_ROOT / "shai" / "parsed"))
    ap.add_argument("--refs", default=None,
                    help="TOML reference file (default: built-in list until K10)")
    args = ap.parse_args()
    corpus = SG.Corpus.load(args.primary_corpus)
    tables = SG.get_tables()
    refs = REFERENCES
    if args.refs:
        import tomllib
        data = tomllib.load(open(args.refs, "rb"))
        refs = {entry["name"]: entry["layout"] for entry in data.get("reference", [])}
    rows = sorted((score(lay, corpus, tables), name, lay) for name, lay in refs.items())
    print(f"{'layout':14s} {'score':>12s}  keys")
    for cost, name, lay in rows:
        print(f"{name:14s} {cost:12.6f}  {lay}")


if __name__ == "__main__":
    main()
