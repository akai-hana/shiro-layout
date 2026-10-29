"""Vectorised loss for large sets of gate instances (cutting-plane rulings)."""
import numpy as np
from . import comfort as cg
from .fingers import KI, FI, NF
from . import gates as G


class GateSet:
    def __init__(self, margin=0.05, weight=0.1):
        self.items = {}          # (keys, frozenset(forb)) -> index
        self.keys2, self.forb2 = [], []
        self.keys3, self.forb3 = [], []
        self.margin, self.weight = margin, weight
        self._dirty = True

    def add(self, keys, forb):
        key = (tuple(keys), frozenset(forb))
        if key in self.items:
            return False
        self.items[key] = len(self.items)
        (self.keys2 if len(keys) == 2 else self.keys3).append((tuple(keys), tuple(sorted(forb))))
        self._dirty = True
        return True

    def __len__(self):
        return len(self.items)

    def _build(self):
        self.pack = {}
        for n, lst in ((2, self.keys2), (3, self.keys3)):
            if not lst:
                self.pack[n] = None
                continue
            ids = np.array([[KI[k] for k in keys] for keys, _ in lst])
            idx = np.indices((NF,) * n)
            masks = np.ones((len(lst),) + (NF,) * n, bool)       # True = legal
            for i, (_, forb) in enumerate(lst):
                for pos, f in forb:
                    masks[i] &= idx[pos] != FI[f]
            pen = np.stack([G.penalty_mask(keys) for keys, _ in lst])
            self.pack[n] = (ids, masks, pen)
        self._dirty = False

    def losses(self, model, tabs):
        """-> (total loss, number violated)"""
        if self._dirty:
            self._build()
        tot, nbad = 0.0, 0
        for n in (2, 3):
            p = self.pack[n]
            if p is None:
                continue
            ids, masks, pen = p
            t = cg.bi_many(tabs, ids[:, 0], ids[:, 1]) if n == 2 else \
                cg.tri_many(model, tabs, ids[:, 0], ids[:, 1], ids[:, 2])
            N = len(ids)
            t = (t + pen).reshape(N, -1)
            m = masks.reshape(N, -1)
            big = 1e6
            t = np.where(np.isfinite(t), t, big)
            ca = np.where(m, t, big).min(1)
            tb = np.sort(np.where(m, big, t), axis=1)[:, :4]
            hinge = np.maximum(0.0, ca[:, None] - tb + self.margin)
            hinge = np.where(tb > big / 2, 0.0, hinge)
            tot += float(hinge.sum())
            nbad += int((hinge[:, 0] > 0).sum())
        return self.weight * tot, nbad
