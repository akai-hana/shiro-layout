"""Shared layout helpers: reference layouts and letter-slot mapping."""

from fingers import KEY_ORDER

LETTERS = "abcdefghijklmnopqrstuvwxyz"

REF = {
    "QWERTY":     "qwertyuiop" "asdfghjkl;" "zxcvbnm,./",
    "Colemak":    "qwfpgjluy;" "arstdhneio" "zxcvbkm,./",
    "Colemak-DH": "qwfpbjluy;" "arstgmneio" "zxcdvkh,./",
    "Canary":     "wlypkzxou;" "crstbfneia" "jvdgqmh/,.",
    "Sturdy":     "vmlcpxfouj" "srtdy.naei" "zkqgwbh';,",
    "Gallium":    "bldcvjfou," "nrtsgyhaei" "xqmwzkp';.",
    "Semimak":    "flhvzqwuoy" "srntkcdeai" "x'bmjpg/,/",
    "Gallium_def": "bldcvjfou," "nrtsgyhaei" "xqmwzkp';.",
}


def dead_of(layout):
    d = {}
    extra = 0
    for i, ch in enumerate(layout):
        if ch not in LETTERS:
            key = ch
            if ch in d:
                extra += 1
                key = f"\x01{extra}"
            d[key] = KEY_ORDER[i]
    return d


def slots_of(ev, layout):
    """Permute letters to slot indices for this layout."""
    slot_pos = {}
    for i, k in enumerate(ev.slots):
        slot_pos[k] = i
    perm = []
    for ch in LETTERS:
        idx = layout.index(ch)
        perm.append(slot_pos[KEY_ORDER[idx]])
    return tuple(perm)


def score_of(ev, layout):
    perm = slots_of(ev, layout)
    return ev.score(perm)
