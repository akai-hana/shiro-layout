"""
ansi.py — True ANSI-staggered keyboard geometry (user: standard ANSI only).

Key-center coordinates in key-width units (x) and key-heights (y). Derived from
standard ANSI key widths: Tab=1.5u, Caps=1.75u, Shift=2.25u, letter keys 1u.

Consequence (verified from the width derivation): the stagger is a UNIFORM
RIGHTWARD drift going down the board — top row 0, home +0.25, bottom +0.75
relative to top — on BOTH hands. It is NOT mirrored. This creates the user's
observed asymmetry: same-integer-column pairs (e.g. E->D left, I->K right) all
have dx=+0.25, but left-hand anatomy wants index RIGHT of middle (aligned) and
right-hand wants index LEFT of middle (crossed). See comfort.lateral_crossing.

Column index (1..10) kept for compat with fingers.py but NO LONGER used for
distance math — use ansi.cx / ansi.cy / ansi.dist.
"""

# Left edge x offsets (key-width units). Center = +0.5.
_X = {
    # top row (y=0)
    "Q": 0.00, "W": 1.00, "E": 2.00, "R": 3.00, "T": 4.00,
    "Y": 5.00, "U": 6.00, "I": 7.00, "O": 8.00, "P": 9.00,
    # home row (y=1), +0.25 from top
    "A": 0.25, "S": 1.25, "D": 2.25, "F": 3.25, "G": 4.25,
    "H": 5.25, "J": 6.25, "K": 7.25, "L": 8.25, ";": 9.25,
    # bottom row (y=2), +0.75 from top (i.e. +0.5 from home)
    "Z": 0.75, "X": 1.75, "C": 2.75, "V": 3.75, "B": 4.75,
    "N": 5.75, "M": 6.75, ",": 7.75, ".": 8.75, "/": 9.75,
}

_Y = {}
for _n in "QWERTYUIOP":
    _Y[_n] = 0.0
for _n in "ASDFGHJKL;":
    _Y[_n] = 1.0
for _n in "ZXCVBNM,./":
    _Y[_n] = 2.0


def x(name):
    """ANSI left-edge x (key widths)."""
    return _X[name]


def y(name):
    """ANSI top-edge y (key heights)."""
    return _Y[name]


def cx(name):
    return _X[name] + 0.5


def cy(name):
    return _Y[name] + 0.5


def row(name):
    """Integer row 0/1/2 (ladder indexing)."""
    return _Y[name]


def dist(a, b):
    """Euclidean distance between key centers (key-width units)."""
    dx = cx(a) - cx(b)
    dy = cy(a) - cy(b)
    return (dx * dx + dy * dy) ** 0.5


def manhattan(a, b):
    return abs(cx(a) - cx(b)) + abs(cy(a) - cy(b))


def stagger_drift(a, b):
    """Horizontal offset between two key centers (key widths)."""
    return abs(cx(a) - cx(b))


if __name__ == "__main__":
    print("ANSI sanity (true geometry):")
    for a, b in [("E", "D"), ("D", "C"), ("R", "F"), ("F", "V"), ("T", "G"),
                 ("G", "B"), ("I", "K"), ("K", ","), ("U", "J"), ("J", "M"),
                 ("Y", "H"), ("N", "M")]:
        print(f"  {a}{b}: dist={dist(a, b):.3f} dx={cx(b) - cx(a):+.3f} "
              f"dy={cy(b) - cy(a):+.1f}")
    print("  (all same-integer-col pairs: dx=+0.25 top->home, +0.5 home->bot)")
