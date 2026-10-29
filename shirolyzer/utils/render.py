"""Render a 30-key layout as staggered ASCII rows."""
from __future__ import annotations

ROW_LENGTH = 10
LAYOUT_LENGTH = 30


def _rows(layout):
    layout = str(layout)
    if len(layout) != LAYOUT_LENGTH:
        raise ValueError(
            f"layout must be {LAYOUT_LENGTH} characters, got {len(layout)}")
    return [layout[i:i + ROW_LENGTH] for i in range(0, LAYOUT_LENGTH, ROW_LENGTH)]


def render(layout, fingers=None):
    """Return the staggered ASCII grid for ``layout``.

    With ``fingers`` (one P/R/M/I token per key), each entry is rendered as the
    layout character followed by that physical key's finger, matching the
    ``fingermap`` format in ``config.yaml``.
    """
    rows = _rows(layout)
    if fingers is not None:
        fingers = list(fingers)
        if len(fingers) != LAYOUT_LENGTH:
            raise ValueError(
                f"fingers must be {LAYOUT_LENGTH} entries, got {len(fingers)}")
        rows = [
            [char + finger for char, finger in
             zip(row, fingers[i * ROW_LENGTH:(i + 1) * ROW_LENGTH])]
            for i, row in enumerate(rows)
        ]
    return "\n".join(
        f"{' ' * i}{' '.join(row[:5])}  {' '.join(row[5:])}"
        for i, row in enumerate(rows)
    )
