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

DEFAULT_FINGER = {"Q":"R","W":"R","E":"M","R":"M","T":"M","Y":"M","U":"M","I":"M","O":"R","P":"R",
                  "A":"P","S":"R","D":"M","F":"I","G":"I","H":"I","J":"I","K":"M","L":"R",";":"P",
                  "Z":"P","X":"R","C":"I","V":"I","B":"I","N":"I","M":"I",",":"R",".":"R","/":"P"}
NODEF_EXEMPT = set()
