#!/usr/bin/env python3
"""Analyze a layout with the calibrated pattern model.

This is a thin entry point; the implementation lives in
``shirolyzer.commands.analyze``.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from shirolyzer.commands.analyze import main


if __name__ == "__main__":
    raise SystemExit(main())
