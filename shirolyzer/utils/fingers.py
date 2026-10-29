"""Finger and key constants for the shirolyzer.

Keys use QWERTY position names.  Fingers are P (pinky), R (ring), M (middle),
and I (index).  Physical geometry, including rest posture and reach, lives in
:mod:`utils.ansi`.
"""
from __future__ import annotations

import numpy as np

ROWS = ["QWERTYUIOP", "ASDFGHJKL;", "ZXCVBNM,./"]
KEYS = [k for r in ROWS for k in r]
KI = {k: i for i, k in enumerate(KEYS)}

FINGERS = "PRMI"
FI = {f: i for i, f in enumerate(FINGERS)}
LENGTH_RANK = np.array([0.0, 1.0, 3.0, 2.0])
NK, NF = len(KEYS), 4

# Populated from the loaded YAML config by utils.config.apply().  Keeping the
# same mapping object lets modules that imported it observe the configured map.
DEFAULT_FINGER: dict[str, str] = {}
NODEF_EXEMPT = set()
