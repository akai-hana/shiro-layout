#!/usr/bin/env python3
"""Score reference layouts against a corpus with the calibrated model."""
import argparse

from ..utils import config as CFG
from ..utils import paths
from ..utils import references as R
from ..utils import scorer as SG
from ..utils.ansi import KEYS, KI


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


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--config", default=None,
                    help="YAML config file (default: config/config.yaml)")
    ap.add_argument("--primary-corpus", default=None,
                    help="parsed corpus dir, corpus dir containing parsed/, or raw .txt/.xz/.zip")
    ap.add_argument("--refs", default=str(paths.REFERENCES_FILE),
                    help="TOML reference file (default: config/references.toml)")
    args = ap.parse_args(argv)
    cfg = CFG.load_and_apply(args.config)
    if args.primary_corpus:
        corpus = SG.Corpus.load(args.primary_corpus)
    else:
        corpus = CFG.load_corpora(cfg, "primary")
    tables = SG.get_tables()
    refs = R.load_reference_map(args.refs)
    rows = sorted((score(lay, corpus, tables), name, lay) for name, lay in refs.items())
    print(f"{'layout':14s} {'score':>12s}  keys")
    for cost, name, lay in rows:
        print(f"{name:14s} {cost:12.6f}  {lay}")


if __name__ == "__main__":
    main()
