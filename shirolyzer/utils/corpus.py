"""N-gram corpus loading for the pattern scorer."""
import json
import os

import numpy as np

SPACE = "@space"


class Corpus:
    """N-gram counts over the keystroke stream (a-z + space)."""

    def __init__(self, letter_freq, bigram_freq, trigram_freq):
        self.L, self.B, self.T = letter_freq, bigram_freq, trigram_freq
        self.chars = [chr(c) for c in range(ord("a"), ord("z") + 1)] + [SPACE]
        self.cid = {c: i for i, c in enumerate(self.chars)}
        self.total_letters = sum(self.L.values())
        self.total_bigrams = sum(self.B.values())
        self.total_trigrams = sum(self.T.values())
        bi, bc, tri, tc = [], [], [], []
        for bg, n in self.B.items():
            if len(bg) == 2 and bg[0] in self.cid and bg[1] in self.cid:
                bi.append((self.cid[bg[0]], self.cid[bg[1]])); bc.append(n)
        for tg, n in self.T.items():
            if len(tg) == 3 and all(c in self.cid for c in tg):
                tri.append(tuple(self.cid[c] for c in tg)); tc.append(n)
        self.bi_ids = np.array(bi, dtype=np.int32)
        self.bi_counts = np.array(bc, dtype=np.float64)
        self.tri_ids = np.array(tri, dtype=np.int32)
        self.tri_counts = np.array(tc, dtype=np.float64)

    @classmethod
    def load(cls, path):
        prefix = os.path.join(path, "") if os.path.isdir(path) else path
        return cls(json.load(open(prefix + "letter_freq.json")),
                   json.load(open(prefix + "bigram_freq.json")),
                   json.load(open(prefix + "trigram_freq.json")))
