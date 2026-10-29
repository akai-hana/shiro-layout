#!/usr/bin/env python3
"""Generate an XKB symbols file for an existing layout.

Examples:
    python3 -m shirolyzer xkb --layout ";ntgfjoehkrpmscuaydilbqvwxz,./" --name shiro
    python3 -m shirolyzer xkb --layout "$(cat layout.txt)" --out /tmp/shiro --install
"""
import argparse
import subprocess
from pathlib import Path

from ..utils import paths
from ..utils import xkb as XKB


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--layout", required=True,
                    help="30-character layout in QWERTY row-major order")
    ap.add_argument("--name", default=None,
                    help="XKB layout name (default: a generated safe name)")
    ap.add_argument("--out", default=None,
                    help="output symbols file (default: xkb/<name>)")
    ap.add_argument("--install", action="store_true",
                    help="install and apply the generated symbols with xkb/install.sh")
    args = ap.parse_args(argv)

    name = args.name or XKB.safe_name(args.layout)
    out = Path(args.out) if args.out else paths.XKB_DIR / name
    XKB.write(out, args.layout, name)
    print(f"wrote {out}")

    if args.install:
        script = paths.XKB_DIR / "install.sh"
        subprocess.run([str(script), str(out), name], check=True)


if __name__ == "__main__":
    main()
