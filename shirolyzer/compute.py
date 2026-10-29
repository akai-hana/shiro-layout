#!/usr/bin/env python3
"""Generate a layout using the calibrated pattern model and current config.

This is a thin entry point; the implementation lives in
``shirolyzer.commands.compute``.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from shirolyzer.commands.compute import main


if __name__ == "__main__":
    raise SystemExit(main())
