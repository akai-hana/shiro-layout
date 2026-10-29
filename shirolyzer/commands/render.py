#!/usr/bin/env python3
"""Render a 30-character layout as a staggered ASCII keyboard.

The default output appends each physical key's configured finger, using the
two-character ``key+finger`` style of ``config/config.yaml``.  Use ``--plain``
for letters only.
"""
import argparse

from ..utils import config as CFG
from ..utils import render as RENDER


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("layout",
                    help="30-character layout in QWERTY row-major order")
    ap.add_argument("--config", default=None,
                    help="YAML config file (default: config/config.yaml)")
    ap.add_argument("--plain", action="store_true",
                    help="omit the finger labels")
    ap.add_argument("--both", action="store_true",
                    help="print both the annotated and plain grids")
    args = ap.parse_args(argv)

    try:
        if args.plain:
            text = RENDER.render(args.layout)
        else:
            cfg = CFG.load(args.config)
            annotated = RENDER.render(args.layout, cfg.finger_tokens)
            if args.both:
                text = f"{annotated}\n\n{RENDER.render(args.layout)}"
            else:
                text = annotated
    except ValueError as exc:
        ap.error(str(exc))
    print(text)


if __name__ == "__main__":
    main()
