"""
scorer.py — Layout evaluator with context-aware alt-fingering.

Because the fingering model is purely positional, the optimal (min-cost)
fingering of every key-trigram can be precomputed ONCE into a 31^3 tensor.
Scoring any letter layout is then a fast count-weighted lookup — which makes
simulated-annealing search practical.

Cost model (calibrated so QWERTY >> Colemak/Gallium under standard fingering):
  per keypress : strength(finger) * (1 + travel from rest)
  bigram       : SFB (row-weighted) | scissors (tolerance-tiered model) |
                 roll reward (in-roll > out-roll) | alternation (small reward)
  trigram      : redirects (weak >> normal) | one-hand continuation (small)
                 | SFS: same finger skipgram, 1u mild / >=2u heavy (rule Z)
Alt fingers cost a small premium so default fingering wins ties (rule T).
"""
import itertools
import numpy as np
from fingers import (KEYS, KEY_ORDER, KEY_INDEX, SPACE, FORDER, STRENGTH,
                     IDEAL_SHARE, REST, scissor_cost, key_effort, cheb)

NKEYS = len(KEY_ORDER)          # 30
SPACE_ID = NKEYS                # pseudo-key id for space
N = NKEYS + 1                   # 31

# ---------------------------------------------------------------- weights
W_SFB_SAME_ROW = 5.0
W_SFB_1ROW    = 6.0
W_SFB_2ROW    = 8.0
W_ROLL_IN     = -0.35           # reward (negative cost)
W_ROLL_OUT    = -0.20
W_ROLL_LEAP   = -0.10           # same-hand, finger-order gap >= 2
W_ALT         = -0.12           # hand alternation
W_REDIRECT     = 0.40           # redirect involving index
W_REDIRECT_WEAK = 1.80          # redirect with no index (weak/bad redirect)
W_CONTINUE     = -0.10          # same-hand direction continuation
W_SFS_1U       = 0.25           # same-finger skipgram, Cheb 1
W_SFS_FAR      = 3.50           # same-finger skipgram, Cheb >= 2  (rule Z)
W_SPACE        = 0.0            # space handled via effort + alternation

# finger ids: P=0 R=1 M=2 I=3 T=4(thumb)
def _finger_ids(name, hand):
    if name == SPACE:
        return [("T", 0.0)]
    return list(KEYS[name].alts)


def _same_hand_bigram(n1, f1, n2, f2):
    k1, k2 = KEYS[n1], KEYS[n2]
    if f1 == f2:
        if n1 == n2:
            return 0.0  # same key repeat (letter repeat): mild, not SFB
        dr = abs(k1.row - k2.row)
        base = W_SFB_SAME_ROW if dr == 0 else (W_SFB_1ROW if dr == 1 else W_SFB_2ROW)
        return base
    # scissors
    sc = scissor_cost(n1, f1, n2, f2)
    # rolls
    o1, o2 = FORDER[f1], FORDER[f2]
    dcol = abs(k1.col - k2.col)
    if o2 > o1:
        reward = W_ROLL_IN if (o2 - o1) == 1 else W_ROLL_LEAP
    else:
        reward = W_ROLL_OUT if (o1 - o2) == 1 else W_ROLL_LEAP
    if dcol >= 3:
        reward *= 0.5
    return sc + reward


def _bigram_cost(n1, f1, n2, f2):
    """Full bigram cost for chosen fingers (names or SPACE)."""
    if n1 == SPACE and n2 == SPACE:
        return 0.0
    if n1 == SPACE or n2 == SPACE:
        return W_ALT  # thumb alternation-ish; effort charged separately
    k1, k2 = KEYS[n1], KEYS[n2]
    if k1.hand != k2.hand:
        return W_ALT
    return _same_hand_bigram(n1, f1, n2, f2)


def _trigram_cost(n1, f1, n2, f2, n3, f3):
    """Trigram-only costs (redirects, continuation, SFS/rule-Z)."""
    cost = 0.0
    # SFS / rule Z: same finger on keys 1 & 3
    if n1 != SPACE and n3 != SPACE and f1 == f3 and f2 != f1:
        k1, k3 = KEYS[n1], KEYS[n3]
        if k1.hand == k3.hand:
            d = cheb((k1.row, k1.col), (k3.row, k3.col))
            cost += W_SFS_1U if d <= 1 else W_SFS_FAR
    # redirects / continuation on same-hand 3-runs (skip thumbs)
    ns = (n1, n2, n3)
    if SPACE in ns:
        return cost
    ks = (KEYS[n1], KEYS[n2], KEYS[n3])
    if ks[0].hand != ks[1].hand or ks[1].hand != ks[2].hand:
        return cost
    o = (FORDER[f1], FORDER[f2], FORDER[f3])
    d1, d2 = o[1] - o[0], o[2] - o[1]
    if d1 == 0 or d2 == 0:
        return cost  # same-finger involvement handled elsewhere
    if d1 * d2 < 0:  # direction reversal = redirect
        idx_involved = "I" in (f1, f2, f3)
        cost += W_REDIRECT if idx_involved else W_REDIRECT_WEAK
    else:
        cost += W_CONTINUE
    return cost


# ---------------------------------------------------------------- tables
def build_tables():
    """BIG[i,j] = min cost bigram key_i->key_j; TRI[i,j,k] likewise.
    Includes per-key alt premiums and keypress effort."""
    BIG = np.zeros((N, N), dtype=np.float64)
    TRI = np.zeros((N, N, N), dtype=np.float64)
    BIG_F = np.zeros((N, N), dtype=np.int8)      # chosen finger ids (argmin)
    TRI_F = np.zeros((N, N, N, 3), dtype=np.int8)

    names = KEY_ORDER + [SPACE]
    eff = {}
    for n in names:
        for f, prem in _finger_ids(n, "L" if n in KEYS and KEYS[n].hand == "L" else "R"):
            eff[(n, f)] = key_effort(n, f) + prem

    FID = {"P": 0, "R": 1, "M": 2, "I": 3, "T": 4}

    for i, n1 in enumerate(names):
        opts1 = _finger_ids(n1, KEYS[n1].hand if n1 in KEYS else "R")
        for j, n2 in enumerate(names):
            opts2 = _finger_ids(n2, KEYS[n2].hand if n2 in KEYS else "R")
            best = None
            for f1, p1 in opts1:
                for f2, p2 in opts2:
                    c = (eff[(n1, f1)] + eff[(n2, f2)] +
                         _bigram_cost(n1, f1, n2, f2))
                    if n1 != n2:
                        c += 0.0
                    if best is None or c < best[0]:
                        best = (c, FID[f1], FID[f2])
            BIG[i, j] = best[0]
            BIG_F[i, j] = best[1] * 5 + best[2]  # packed

    for i, n1 in enumerate(names):
        opts1 = _finger_ids(n1, KEYS[n1].hand if n1 in KEYS else "R")
        for j, n2 in enumerate(names):
            opts2 = _finger_ids(n2, KEYS[n2].hand if n2 in KEYS else "R")
            for k, n3 in enumerate(names):
                opts3 = _finger_ids(n3, KEYS[n3].hand if n3 in KEYS else "R")
                best = None
                for f1, p1 in opts1:
                    for f2, p2 in opts2:
                        for f3, p3 in opts3:
                            c = (eff[(n1, f1)] + eff[(n2, f2)] + eff[(n3, f3)] +
                                 _bigram_cost(n1, f1, n2, f2) +
                                 _bigram_cost(n2, f2, n3, f3) +
                                 _trigram_cost(n1, f1, n2, f2, n3, f3))
                            if best is None or c < best[0]:
                                best = (c, FID[f1], FID[f2], FID[f3])
                TRI[i, j, k] = best[0]
                TRI_F[i, j, k] = best[1:]
    return BIG, TRI, BIG_F, TRI_F


TABLES = None


def get_tables():
    global TABLES
    if TABLES is None:
        TABLES = build_tables()
    return TABLES


# ---------------------------------------------------------------- default
# Standard (no-alt) fingering = default finger only.
def build_default_tables():
    BIGd = np.zeros((N, N))
    TRI_d = np.zeros((N, N, N))
    names = KEY_ORDER + [SPACE]
    eff = {n: key_effort(n, _finger_ids(n, "L")[0][0] if n in KEYS and KEYS[n].hand == "L"
                         else _finger_ids(n, "R")[0][0]) for n in names}
    # default finger = first entry of alts tuple (premium 0)
    def dflt(n):
        return _finger_ids(n, "L")[0][0] if (n in KEYS and KEYS[n].hand == "L") \
            else _finger_ids(n, "R")[0][0]
    for i, n1 in enumerate(names):
        for j, n2 in enumerate(names):
            f1, f2 = dflt(n1), dflt(n2)
            BIGd[i, j] = eff[n1] + eff[n2] + _bigram_cost(n1, f1, n2, f2)
            for k, n3 in enumerate(names):
                f3 = dflt(n3)
                TRI_d[i, j, k] = (BIGd[i, j] + eff[n3] +
                                  _bigram_cost(n2, f2, n3, f3) +
                                  _trigram_cost(n1, f1, n2, f2, n3, f3))
    return BIGd, TRI_d


DEFAULT_TABLES = None


def get_default_tables():
    global DEFAULT_TABLES
    if DEFAULT_TABLES is None:
        DEFAULT_TABLES = build_default_tables()
    return DEFAULT_TABLES


# ---------------------------------------------------------------- corpus
class Corpus:
    """N-gram counts over the keystroke stream (a-z + space)."""

    def __init__(self, letter_freq, bigram_freq, trigram_freq, space_frac=0.1777):
        self.L = letter_freq
        self.B = bigram_freq
        self.T = trigram_freq
        self.space_frac = space_frac
        self._prepare()

    def _prepare(self):
        chars = [chr(c) for c in range(ord("a"), ord("z") + 1)] + [SPACE]
        self.chars = chars
        self.cid = {c: i for i, c in enumerate(chars)}
        self.total_letters = sum(self.L.values())
        self.total_bigrams = sum(self.B.values())
        self.total_trigrams = sum(self.T.values())

        # bigram counts incl. space transitions (estimated from position data
        # if not present in B): letter->space ~ letter final share; space->letter
        # ~ letter initial share. We approximate: count(letter, space) =
        # final_share[letter] * n_spaces etc. Caller passes enriched B/T with
        # space keys already if available; otherwise skip space here.
        self.bi_ids, self.bi_counts = [], []
        for bg, cnt in self.B.items():
            if len(bg) != 2:
                continue
            if bg[0] not in self.cid or bg[1] not in self.cid:
                continue
            self.bi_ids.append((self.cid[bg[0]], self.cid[bg[1]]))
            self.bi_counts.append(cnt)
        self.bi_ids = np.array(self.bi_ids, dtype=np.int32)
        self.bi_counts = np.array(self.bi_counts, dtype=np.float64)

        self.tri_ids, self.tri_counts = [], []
        for tg, cnt in self.T.items():
            if len(tg) != 3:
                continue
            if any(c not in self.cid for c in tg):
                continue
            self.tri_ids.append(tuple(self.cid[c] for c in tg))
            self.tri_counts.append(cnt)
        self.tri_ids = np.array(self.tri_ids, dtype=np.int32)
        self.tri_counts = np.array(self.tri_counts, dtype=np.float64)

    @classmethod
    def load(cls, path_prefix):
        import json
        L = json.load(open(path_prefix + "letter_freq.json"))
        B = json.load(open(path_prefix + "bigram_freq.json"))
        T = json.load(open(path_prefix + "trigram_freq.json"))
        return cls(L, B, T)


def layout_to_ids(layout30):
    """layout30: string of 30 chars (letter or punct) in KEY_ORDER.
    Returns list mapping letter->key id. Punct chars pass through as their
    key's id when found in the layout; unknown -> ignored by scorer via
    mapping only for a-z."""
    m = {}
    for i, ch in enumerate(layout30):
        m[ch] = i
    return m


def score_layout(layout30, corpus, tables=None, default_tables=None):
    """layout30: 30-char string. Returns metrics dict + score (lower=better cost)."""
    if tables is None:
        tables = get_tables()
    BIG, TRI = tables[0], tables[1]
    if default_tables is None:
        default_tables = get_default_tables()
    BIGd, TRId = default_tables

    m = layout_to_ids(layout30)
    # map corpus ids (chars) -> layout key ids
    remap = np.full(len(corpus.chars), -1, dtype=np.int32)
    for ci, ch in enumerate(corpus.chars):
        if ch == SPACE:
            remap[ci] = SPACE_ID
        elif ch in m:
            remap[ci] = m[ch]

    bi = remap[corpus.bi_ids[:, 0]] * N + remap[corpus.bi_ids[:, 1]]
    tri = (remap[corpus.tri_ids[:, 0]] * N * N +
           remap[corpus.tri_ids[:, 1]] * N +
           remap[corpus.tri_ids[:, 2]])
    ok_b = bi >= 0
    ok_t = tri >= 0
    cost_bi = float((BIG.ravel()[bi[ok_b]] * corpus.bi_counts[ok_b]).sum())
    cost_tri = float((TRI.ravel()[tri[ok_t]] * corpus.tri_counts[ok_t]).sum())
    n_bi = float(corpus.bi_counts[ok_b].sum())
    n_tri = float(corpus.tri_counts[ok_t].sum())

    cost_bi_d = float((BIGd.ravel()[bi[ok_b]] * corpus.bi_counts[ok_b]).sum())
    cost_tri_d = float((TRId.ravel()[tri[ok_t]] * corpus.tri_counts[ok_t]).sum())

    # finger usage from letter freq (default finger) + balance penalty
    usage = {"L": {f: 0.0 for f in "PRMI"}, "R": {f: 0.0 for f in "PRMI"}}
    for ch, cnt in corpus.L.items():
        if ch not in m:
            continue
        k = KEY_ORDER[m[ch]]
        f = KEYS[k].alts[0][0]
        usage[KEYS[k].hand][f] += cnt
    tot = {h: sum(usage[h].values()) for h in "LR"}
    bal_pen = 0.0
    for h in "LR":
        if tot[h] == 0:
            continue
        for f in "PRMI":
            share = usage[h][f] / tot[h]
            bal_pen += ((share - IDEAL_SHARE[f]) / IDEAL_SHARE[f]) ** 2
    bal_pen *= 1.5  # weight

    score = (cost_bi + cost_tri) / max(n_tri, 1) + bal_pen
    return {
        "score": score,
        "cost_bigram_per_tri": cost_bi / max(n_tri, 1),
        "cost_trigram_per_tri": cost_tri / max(n_tri, 1),
        "cost_default_per_tri": (cost_bi_d + cost_tri_d) / max(n_tri, 1),
        "balance_penalty": bal_pen,
        "usage": usage,
    }
