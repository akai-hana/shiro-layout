"""Physical-layout geometry registry.

New one-word layouts (for example ``iso`` or ``matrix``) can be added as
sibling packages with the same public geometry API and registered here without
changing callers.
"""
from . import ansi

PHYSICAL_LAYOUTS = {"ansi": ansi}
physical_layout = "ansi"


def get_layout(name=None):
    name = name or physical_layout
    try:
        return PHYSICAL_LAYOUTS[name]
    except KeyError:
        raise ValueError(f"unknown physical layout: {name!r}") from None


def register_layout(name, module):
    PHYSICAL_LAYOUTS[name] = module


from .ansi import (  # noqa: E402,F401
    ROWS, KEYS, KI, FINGERS, FI, NK, NF, ROW_OFFSET,
    REST, REST_KEYS, U, V, H, ROWI, CX, REST_U, REST_V, INNER_SEP,
    DU, DV, APPROVED, REACH, LISTED, UNLISTED,
)
