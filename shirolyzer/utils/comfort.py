"""Pattern-based comfort model for the shirolyzer.

All physical constants live in :mod:`utils.fingers`.  Every cost is a smooth
function of key displacement, scissor strain, lateral separation, roll/leap
direction, and trigram context.
"""
from __future__ import annotations

import itertools
import math
from dataclasses import dataclass, field

import numpy as np

from .fingers import *
from .fingers import INNER_SEP

# ---------------------------------------------------------------- parameters
# name: (default, lo, hi, group)
PARAMS = {
    # single-key travel
    "e_dn":      (0.14, 0.0, 0.8, "travel"),
    "e_up_I":    (0.15, 0.0, 1.2, "travel"),
    "e_up_P":    (1.00, 0.0, 3.0, "travel"),
    "e_lat":     (0.20, 0.0, 0.8, "travel"),
    "e_lat2":    (0.10, 0.0, 1.0, "travel"),
    "e_unl":     (0.60, 0.0, 3.0, "travel"),    # surcharge: reachable but never listed by the user
    "e_dn2":     (0.20, 0.0, 1.5, "travel"),    # extra for a 2-row drop (full-span reach down)
    "e_ov":      (0.30, 0.0, 2.0, "travel"),    # finger reaches past the INNER neighbour's rest column
    # vertical strain
    "p":         (2.0, 1.0, 3.0, "vert"),
    "a_MI":      (0.15, 0.0, 3.0, "vert"),
    "a_RM":      (1.50, 0.0, 6.0, "vert"),
    "a_PR":      (1.50, 0.0, 6.0, "vert"),
    "a_RI":      (1.00, 0.0, 6.0, "vert"),
    "a_PM":      (0.50, 0.0, 6.0, "vert"),
    "a_PI":      (0.50, 0.0, 6.0, "vert"),
    "b_MI":      (0.20, 0.0, 2.0, "vert"),
    "c_MIrev":   (3.00, 0.0, 6.0, "vert"),    # M/I full-span reverse scissor ('barely possible')
    # lateral crowding / crossing / spread
    "m_PR":      (0.50, -0.5, 1.5, "lat"),
    "m_RM":      (0.25, -0.5, 1.5, "lat"),
    "m_MI":      (0.25, -0.5, 1.5, "lat"),
    "m_skip":    (0.75, 0.0, 1.5, "lat"),
    "kappa_MI":  (0.40, 0.0, 1.0, "lat"),     # row-gap relief of the min separation, M/I
    "kappa_o":   (0.00, 0.0, 1.0, "lat"),     # ... all other finger pairs
    "varsig":    (0.35, 0.0, 1.0, "lat"),     # extra lateral room a scissor needs, per unit g
    "lam_x":     (0.30, 0.0, 3.0, "lat"),     # soft cost per key-width of crossing
    "lam_prox":  (0.50, 0.0, 3.0, "lat"),     # soft cost of clamping adjacent fingers together
    "lam_spread": (1.00, 0.0, 3.0, "lat"),
    "w_spread":  (1.00, 0.0, 2.0, "lat"),
    # rolls / leaps
    "r_MI":      (-0.275, -0.8, 0.2, "roll"),
    "r_RM_in":   (-0.35, -0.8, 0.2, "roll"),     # ring -> middle
    "r_RM_out":  (-0.20, -0.8, 0.2, "roll"),     # middle -> ring
    "r_PR_out":  (-0.14, -0.8, 0.2, "roll"),     # ring -> pinky  (good)
    "r_PR_in":   (0.06, -0.4, 0.8, "roll"),      # pinky -> ring  (awkward)
    "leap2":     (0.15, 0.0, 1.5, "roll"),
    "leap3":     (0.35, 0.0, 2.0, "roll"),
    "leap_P":    (0.15, 0.0, 1.5, "roll"),
    "gamma13":   (0.50, 0.0, 1.5, "tri"),
    "e_nd":      (0.30, 0.0, 1.0, "travel"),    # non-default finger on an ordinary key ('defaults first'); Q/S/... middle alts are cheap by design
    "nd_waive":  (1.0, 0.0, 1.0, "travel"),     # a key whose default finger is busy on the neighbouring key pays no default-finger penalty
    "shift_thr": (0.6, 0.0, 1.5, "travel"),     # only displacement shared beyond this many columns counts as a hand shift
    "shift_disc": (0.30, 0.0, 1.0, "travel"),   # two fingers displaced sideways the same way = the hand shifts: refund this share of the shared lateral effort
    "e_ndgap":   (0.50, 0.0, 1.0, "travel"),    # leaving a default finger also forfeits this share of the travel saving it offers
    "e_nd_rest": (0.30, 0.0, 1.0, "travel"),    # non-rest finger on a REST key (A W E F ; O I J): zero travel from defaults is preferred
    "scis_lat":  (0.50, 0.0, 4.0, "vert"),      # half-span ring/middle scissor gets worse as the hand is displaced sideways
    "scis_dir":  (0.30, 0.0, 0.9, "vert"),      # entering a scissor (second finger = the longer one) costs more than leaving it
    "e_upcol":   (0.30, 0.0, 1.5, "travel"),    # index straight UP its own column (U, R) is worse than diagonal-up (T, Y)
    "gamma13_alt": (0.80, 0.0, 1.5, "tri"),     # same: keys 1&3 on one hand with the other hand in between
}
PNAMES = list(PARAMS)
THETA0 = np.array([PARAMS[n][0] for n in PNAMES])
LO = np.array([PARAMS[n][1] for n in PNAMES])
HI = np.array([PARAMS[n][2] for n in PNAMES])

# constants that are the user's explicit declarations, not fitted
B_IMP = 9.0              # "impossible" -- above the worst SFB (8.0), so any
                         # typeable alternative, even an SFB, beats it
G_IMP = 1.5              # full-span (2 rows) M/R reverse scissor, measured as g from the arched rest
G_IMP_RP = 1.5           # R/P: USER 2026-10-27: ring on the bottom row with the pinky on the home row (X+A, comma/period+;) is practically impossible
DELTA_IMP = 0.30         # deficit (key widths) beyond which fingers can't fit
SPREAD_LIM = 2.0         # USER 2026-10-29: ring+middle on ;-U ("borderline impossible"): two adjacent fingers more than
SPREAD_IMP = 6.0         # this many key widths wider than their rest spacing cost SPREAD_IMP (between "hard" and "impossible")
IP_SMIN = 1.9            # USER 2026-10-28: index + pinky with only one column between the keys (X-A, s=1.5) is "madness".
                         # Same-hand index/pinky pairs need at least this key separation (in key widths).
TAU = 0.05               # sharpness of the steps
W_ALT = -0.12            # hand alternation
PAIR_CAP = 12.0          # one pair can be 'impossible' only once (no double counting)
W_REPEAT, W_REPEAT_SWAP = 0.6, 0.5
W_REDIRECT, W_REDIRECT_WEAK, W_CONTINUE = 0.30, 1.80, -0.10
W_SFS_1U, W_SFS_FAR = 2.50, 3.50
ROLL_D0, ROLL_D1 = 1.25, 1.0       # roll reward fades between dist 1.25..2.25


def sfb_cost(a, b):
    """Same finger, different keys: 5 / 6 / 8 by row gap, +0.3 if reaching down."""
    dr = abs(V[a] - V[b])
    c = 5.0 + 0.5 * dr + 0.5 * dr * dr
    if V[b] > V[a] and dr >= 1:
        c += 0.3
    return c


def _sig(x):
    return 1.0 / (1.0 + np.exp(-np.clip(x, -50, 50)))


# --------------------------------------------------- pair feature precompute
def _build_pair_features():
    a = np.arange(NK)[:, None, None, None]
    fa = np.arange(NF)[None, :, None, None]
    b = np.arange(NK)[None, None, :, None]
    fb = np.arange(NF)[None, None, None, :]
    a, fa, b, fb = np.broadcast_arrays(a, fa, b, fb)
    same_hand = H[a] == H[b]
    diff_f = fa != fb
    # longer finger = higher LENGTH_RANK
    a_longer = LENGTH_RANK[fa] > LENGTH_RANK[fb]
    dv_a, dv_b = DV[a, fa], DV[b, fb]
    g = np.where(a_longer, dv_a - dv_b, dv_b - dv_a)
    inner_is_a = fa > fb                               # finger index = lateral order
    s = np.where(inner_is_a, U[a] - U[b], U[b] - U[a])
    drow = np.abs(V[a] - V[b])
    dist = np.hypot(CX[a] - CX[b], V[a] - V[b])
    lo_f, hi_f = np.minimum(fa, fb), np.maximum(fa, fb)
    nadj = hi_f - lo_f
    direction = np.sign(fb - fa)                       # +1 toward index
    # rest separation inner-outer for this finger pair
    hand = H[a]
    rest_sep = REST_U[hand, hi_f] - REST_U[hand, lo_f]
    return dict(a=a, fa=fa, b=b, fb=fb, same_hand=same_hand, diff_f=diff_f,
                g=g, s=s, drow=drow, dist=dist, lo_f=lo_f, hi_f=hi_f, nadj=nadj,
                direction=direction, rest_sep=rest_sep)


PF = _build_pair_features()
SFB_T = np.vectorize(sfb_cost)(PF["a"], PF["b"])        # theta-independent
_PAIR_ID = {(0, 1): "PR", (1, 2): "RM", (2, 3): "MI", (0, 2): "PM", (1, 3): "RI", (0, 3): "PI"}


@dataclass
class Model:
    theta: np.ndarray = field(default_factory=lambda: THETA0.copy())
    disallow_full_MR: bool = True    # user's new rule; False = priced, "barely possible"

    def p(self, name):
        return self.theta[PNAMES.index(name)]

    # ------------------------------------------------------------- single key
    def key_cost(self):
        e = np.zeros((NK, NF))
        up_pen = np.array([self.p("e_up_P"), 0.0, 0.0, self.p("e_up_I")])
        for f in range(NF):
            dv, du = DV[:, f], DU[:, f]
            colup = (self.p("e_upcol") * np.maximum(0.0, 1.0 - np.abs(du) / 0.5) * (dv < 0)) if f == 3 else 0.0
            e[:, f] = (colup + self.p("e_dn") * np.maximum(dv, 0)
                       + up_pen[f] * np.maximum(-dv, 0)
                       + self.p("e_lat") * np.abs(du)
                       + self.p("e_lat2") * np.maximum(np.abs(du) - 1.0, 0) ** 2
                       + self.p("e_dn2") * np.maximum(dv - 1.0, 0) ** 2
                       + self.p("e_ov") * (np.maximum(du - INNER_SEP[H, f], 0) if f < 3 else 0.0))
        e = e + self.p("e_unl") * UNLISTED
        nd = np.zeros((NK, NF))
        for k, name in enumerate(KEYS):
            if name in NODEF_EXEMPT:
                continue
            for f in range(NF):
                nd[k, f] = 0.0 if FINGERS[f] == DEFAULT_FINGER[name] else 1.0
        ndr = np.zeros((NK, NF))
        for k, name in enumerate(KEYS):
            if name in REST_KEYS:
                ndr[k] = nd[k]
                nd[k] = 0.0
        dflt = np.array([FINGERS.index(DEFAULT_FINGER[n]) for n in KEYS])
        e_def = e[np.arange(NK), dflt][:, None]
        gap = np.where(np.isfinite(e_def) & np.isfinite(e), np.maximum(e_def - e, 0.0), 0.0) * ((nd + ndr) > 0)
        self._ND = self.p("e_nd") * nd + self.p("e_ndgap") * gap          # waivable part (rest-key penalty is not)
        e = e + self._ND + self.p("e_nd_rest") * ndr
        e = np.where(REACH, e, np.inf)
        return e

    # ---------------------------------------------------------- pair terms
    def pair_parts(self):
        """Return dict of (NK,NF,NK,NF) arrays for each cost component
        (same-hand, different-finger entries only; others 0)."""
        P = PF
        g, s, drow = P["g"], P["s"], P["drow"]
        lo_f, hi_f, nadj = P["lo_f"], P["hi_f"], P["nadj"]
        sh = P["same_hand"] & P["diff_f"]
        p = self.p("p")

        # vertical strain, alpha by finger pair
        alpha = np.zeros_like(g)
        for (x, y), nm in _PAIR_ID.items():
            alpha = np.where((lo_f == x) & (hi_f == y), self.p("a_" + nm), alpha)
        beta = np.where((lo_f == 2) & (hi_f == 3), self.p("b_MI"), 0.0)
        second_longer = LENGTH_RANK[P["fb"]] > LENGTH_RANK[P["fa"]]
        adj_MR_RP = (nadj == 1) & (lo_f <= 1)      # ring/middle and pinky/ring only (user 2026-10-28: K-; and ;-K must price alike)
        dirf = 1.0 + np.where(adj_MR_RP, self.p("scis_dir"), 0.0) * np.where(second_longer, 1.0, -1.0)
        latf = 1.0 + np.where(adj_MR_RP, self.p("scis_lat"), 0.0) * 0.5 * (np.abs(DU[P["a"], P["fa"]]) + np.abs(DU[P["b"], P["fb"]]))
        vert = np.where(g > 0, alpha * np.abs(g) ** p * dirf * latf, beta * np.abs(g) ** p)
        # hard step: full-span reverse M/R, R/P  (user: disallow)
        # 'full span' = the two KEYS are two rows apart with the longer finger lower.  Measured
        # from the arched rest, that is g = 2 for middle/ring (both rest on the top row) but
        # g = 3 for ring/pinky (ring rests one row above the pinky), so the threshold differs.
        thr = np.where(lo_f == 0, G_IMP_RP, G_IMP)
        if self.disallow_full_MR:
            step = B_IMP * _sig((g - thr) / TAU)
        else:   # keep R/P impossible, M/R "barely possible" (priced, below SFB)
            step = np.where(lo_f == 0, B_IMP * _sig((g - thr) / TAU),
                            5.85 * _sig((g - thr) / TAU))
        vert = vert + np.where(adj_MR_RP, step, 0.0)
        vert = vert + np.where((lo_f == 2) & (hi_f == 3),
                               self.p("c_MIrev") * _sig((-g - 0.5) / TAU), 0.0)

        # crowding / crossing / stacking.  One inequality: the key separation s
        # must be at least  need = (sum of finger gaps spanned + skipped-finger
        # allowance) - row-gap relief + varsig * scissor strain.
        m = np.array([self.p("m_PR"), self.p("m_RM"), self.p("m_MI")])
        cover = np.zeros(g.shape)
        for gap in range(3):
            cover = cover + np.where((lo_f <= gap) & (hi_f > gap), m[gap], 0.0)
        cover = cover + self.p("m_skip") * np.maximum(nadj - 1, 0)
        kap = np.where((lo_f == 2) & (hi_f == 3), self.p("kappa_MI"), self.p("kappa_o"))
        need = cover - kap * np.minimum(drow, 2.0) + self.p("varsig") * np.maximum(g, 0)
        deficit = need - s
        prox = np.where(nadj == 1, np.maximum(0.8 * P["rest_sep"] - s, 0), 0.0)
        ipstep = np.where((lo_f == 0) & (hi_f == 3), B_IMP * _sig((IP_SMIN - s) / TAU), 0.0)
        cross = (B_IMP * _sig((deficit - DELTA_IMP) / TAU) + ipstep
                 + self.p("lam_x") * np.maximum(-s, 0)
                 + self.p("lam_prox") * prox)

        # spread
        excess = s - (P["rest_sep"] + self.p("w_spread"))
        spread = self.p("lam_spread") * np.maximum(excess, 0)
        spread = spread + np.where(nadj == 1, SPREAD_IMP * _sig((s - P["rest_sep"] - SPREAD_LIM) / TAU), 0.0)

        # rolls (adjacent) / leaps (gap>=2)
        d = P["direction"]
        rw = np.zeros_like(g)
        rw = np.where((lo_f == 2) & (hi_f == 3), self.p("r_MI"), rw)
        rw = np.where((lo_f == 1) & (hi_f == 2) & (d > 0), self.p("r_RM_in"), rw)
        rw = np.where((lo_f == 1) & (hi_f == 2) & (d < 0), self.p("r_RM_out"), rw)
        rw = np.where((lo_f == 0) & (hi_f == 1) & (d > 0), self.p("r_PR_in"), rw)
        rw = np.where((lo_f == 0) & (hi_f == 1) & (d < 0), self.p("r_PR_out"), rw)
        fade = np.clip(1.0 - np.maximum(P["dist"] - ROLL_D0, 0) / ROLL_D1, 0.0, 1.0)
        supp = np.where(deficit > 0.2, 0.25, 1.0)
        roll = np.where(nadj == 1, rw * fade * supp, 0.0)
        leap = np.where(nadj == 2, self.p("leap2"), 0.0) + np.where(nadj == 3, self.p("leap3"), 0.0)
        leap = leap + np.where((nadj >= 2) & (lo_f == 0), self.p("leap_P"), 0.0)

        z = np.zeros_like(g)
        self.key_cost()
        dflt_i = np.array([FINGERS.index(DEFAULT_FINGER[n]) for n in KEYS])
        nd_b = np.where(np.isfinite(self._ND[P["b"], P["fb"]]), self._ND[P["b"], P["fb"]], 0.0)
        nd_a = np.where(np.isfinite(self._ND[P["a"], P["fa"]]), self._ND[P["a"], P["fa"]], 0.0)
        waive = -self.p("nd_waive") * (nd_b * (P["fa"] == dflt_i[P["b"]]) + nd_a * (P["fb"] == dflt_i[P["a"]])) * ((vert + cross) < 4.0)
        dua, dub = DU[P["a"], P["fa"]], DU[P["b"], P["fb"]]
        shared = np.where(np.sign(dua) * np.sign(dub) > 0, np.minimum(np.abs(dua), np.abs(dub)), 0.0)
        shift = -self.p("shift_disc") * self.p("e_lat") * np.maximum(shared - self.p("shift_thr"), 0.0)
        return {k: np.where(sh, v, z) for k, v in
                dict(vert=vert, cross=cross, spread=spread, roll=roll, leap=leap, shift=shift + waive).items()}

    def pair_table(self):
        """Full pair cost (no single-key effort): (NK,NF,NK,NF)."""
        parts = self.pair_parts()
        tot = np.minimum(sum(parts.values()), PAIR_CAP)
        P = PF
        a, b, fa, fb = P["a"], P["b"], P["fa"], P["fb"]
        out = np.where(P["same_hand"], tot, W_ALT)
        # same finger / same key special classes
        sfb = SFB_T
        same_f = P["same_hand"] & (fa == fb) & (a != b)
        out = np.where(same_f, sfb, out)
        rep = (a == b)
        out = np.where(rep, W_REPEAT + np.where(fa != fb, W_REPEAT_SWAP, 0.0), out)
        # unreachable finger/key combos
        reach = REACH[a, fa] & REACH[b, fb]
        return np.where(reach, out, np.inf), parts

    # ------------------------------------------------------------ evaluators
    def tables(self):
        E = self.key_cost()
        PT, parts = self.pair_table()
        T2 = E[:, :, None, None] + E[None, None, :, :] + PT
        strain = parts["vert"] + parts["cross"] + parts["spread"]
        return dict(E=E, PT=PT, T2=T2, parts=parts, strain=strain)


# --------------------------------------------------------------- public API
def best_bigram(model_tables, k1, k2, topn=3):
    """k1, k2 = key names. Returns list of (cost, 'XY' fingering)."""
    a, b = KI[k1], KI[k2]
    t = model_tables["T2"][a, :, b, :]
    rows = []
    for fa in range(NF):
        for fb in range(NF):
            if a == b and fa != fb:
                continue
            c = t[fa, fb]
            if np.isfinite(c):
                rows.append((float(c), FINGERS[fa] + FINGERS[fb]))
    rows.sort()
    return rows[:topn]


def _hand_masks():
    sh = (H[:, None] == H[None, :])
    return sh


_SAMEH = _hand_masks()
_DXY = np.hypot(CX[:, None] - CX[None, :], V[:, None] - V[None, :])


def tri_tensor(model, tabs, a=None):
    """Cost of every fingering of every key trigram.
    Returns (NK,NK,NK,NF,NF,NF) if a is None, else (NK,NK,NF,NF,NF) for first key a.
    inf = unreachable / inconsistent repeat."""
    E, PT, strain = tabs["E"], tabs["PT"], tabs["strain"]
    g13 = model.p("gamma13")
    fs = np.arange(NF)
    d12 = fs[None, :] - fs[:, None]                       # f2-f1   (NF,NF)
    out = []
    rng = range(NK) if a is None else [a]
    for ka in rng:
        # axes: b, c, f1, f2, f3
        # PT[ka] has shape (f1, b, f2): index it explicitly
        P12 = PT[ka]                                        # (f1, b, f2)
        P12 = np.transpose(P12, (1, 0, 2))                  # (b, f1, f2)
        P23 = np.transpose(PT, (0, 2, 1, 3))                # (b, c, f2, f3)  from PT[b,f2,c,f3]
        cost = (E[ka][None, None, :, None, None]
                + E[:, None, None, :, None]
                + E[None, :, None, None, :]
                + P12[:, None, :, :, None]
                + P23[:, :, None, :, :])
        sh12 = _SAMEH[ka][:, None]
        sh23 = _SAMEH                                       # (b, c)
        same = (sh12 & sh23)[:, :, None, None, None]
        D1 = d12[None, None, :, :, None]                    # f2-f1
        D2 = d12[None, None, None, :, :]                    # f3-f2
        d1 = np.broadcast_to(D1, cost.shape); d2 = np.broadcast_to(D2, cost.shape)
        has_p = ((fs[None, None, :, None, None] == 0) | (fs[None, None, None, :, None] == 0)
                 | (fs[None, None, None, None, :] == 0))
        turn = (d1 * d2 < 0)
        cont = (d1 * d2 > 0)
        add = np.where(same & turn, np.where(has_p, W_REDIRECT_WEAK, W_REDIRECT), 0.0)
        add = add + np.where(same & cont, W_CONTINUE, 0.0)
        # gamma13 strain between key1 and key3
        S13 = np.transpose(strain[ka], (1, 0, 2))           # (c, f1, f3)
        S13 = np.transpose(S13, (0, 1, 2))
        s13 = S13[None, :, :, None, :]                      # (1, c, f1, 1, f3)
        diff13 = (fs[None, None, :, None, None] != fs[None, None, None, None, :]) \
                 & (ka != np.arange(NK))[None, :, None, None, None]
        add = add + np.where(same & diff13, g13 * np.where(np.isfinite(s13), s13, 0.0), 0.0)
        # same-finger-skip
        f1e = fs[None, None, :, None, None]; f3e = fs[None, None, None, None, :]; f2e = fs[None, None, None, :, None]
        sh13 = _SAMEH[ka][None, :, None, None, None]
        sfs_mask = (sh13 & (f1e == f3e) & (np.arange(NK) != ka)[None, :, None, None, None]
                    & ~(sh12[:, :, None, None, None] & (f2e == f1e)))
        sfs_cost = np.where(_DXY[ka] <= 1.3, W_SFS_1U, W_SFS_FAR)[None, :, None, None, None]
        add = add + np.where(sfs_mask, sfs_cost, 0.0)
        cost = cost + add
        # repeat constraints
        bb = np.arange(NK)[:, None, None, None, None]; cc = np.arange(NK)[None, :, None, None, None]
        bad = (((ka == bb) & (f1e != f2e)) | ((bb == cc) & (f2e != f3e)) | ((ka == cc) & (f1e != f3e)))
        cost = np.where(bad, np.inf, cost)
        out.append(cost)
    return np.stack(out) if a is None else out[0]


_FS = np.arange(NF)
_D12 = _FS[None, :] - _FS[:, None]          # f2 - f1
_HASP = ((_FS[:, None, None] == 0) | (_FS[None, :, None] == 0) | (_FS[None, None, :] == 0))
_TURN = (_D12[:, :, None] * _D12[None, :, :] < 0)     # (f1,f2,f3): d1*d2<0
_CONT = (_D12[:, :, None] * _D12[None, :, :] > 0)
_NE13 = (_FS[:, None, None] != _FS[None, None, :]) & np.ones((1, NF, 1), bool)
_SAME13F = (_FS[:, None, None] == _FS[None, None, :]) & np.ones((1, NF, 1), bool)
_F1EQF2 = (_FS[:, None, None] == _FS[None, :, None]) & np.ones((1, 1, NF), bool)


def tri_many(model, tabs, A, B, C):
    """Vectorised over N key trigrams: (N,NF,NF,NF) fingering costs (inf = invalid)."""
    A, B, C = (np.asarray(x, int) for x in (A, B, C))
    E, PT, strain = tabs["E"], tabs["PT"], tabs["strain"]
    F = _FS
    P12 = PT[A[:, None, None], F[None, :, None], B[:, None, None], F[None, None, :]]   # N,f1,f2
    P23 = PT[B[:, None, None], F[None, :, None], C[:, None, None], F[None, None, :]]   # N,f2,f3
    cost = (E[A][:, :, None, None] + E[B][:, None, :, None] + E[C][:, None, None, :]
            + P12[:, :, :, None] + P23[:, None, :, :])
    same = (_SAMEH[A, B] & _SAMEH[B, C])[:, None, None, None]
    add = np.where(_TURN, np.where(_HASP, W_REDIRECT_WEAK, W_REDIRECT), 0.0) \
        + np.where(_CONT, W_CONTINUE, 0.0)
    s13 = strain[A[:, None, None], F[None, :, None], C[:, None, None], F[None, None, :]]  # N,f1,f3
    s13 = np.where(np.isfinite(s13), s13, 0.0)
    g = model.p("gamma13") * s13[:, :, None, :] * _NE13[None] * (A != C)[:, None, None, None]
    cost = cost + np.where(same, add[None] + g, 0.0)
    alt13 = (_SAMEH[A, C] & ~_SAMEH[A, B] & (A != C))[:, None, None, None]
    g_alt = model.p("gamma13_alt") * s13[:, :, None, :] * _NE13[None]
    cost = cost + np.where(alt13, g_alt, 0.0)
    d = _DXY[A, C]
    sfs = np.where(d <= 1.3, W_SFS_1U, W_SFS_FAR)[:, None, None, None]
    m13 = _SAME13F[None] & ~((_SAMEH[A, B])[:, None, None, None] & _F1EQF2[None])
    cost = cost + np.where(m13 & (_SAMEH[A, C] & (A != C))[:, None, None, None], sfs, 0.0)
    ne12 = (_FS[:, None, None] != _FS[None, :, None])[None]
    ne23 = (_FS[None, :, None] != _FS[None, None, :])[None]
    ne13 = (_FS[:, None, None] != _FS[None, None, :])[None]
    bad = (((A == B)[:, None, None, None] & ne12) | ((B == C)[:, None, None, None] & ne23)
           | ((A == C)[:, None, None, None] & ne13))
    return np.where(bad, np.inf, cost)


def tri_block(model, tabs, a, b, c):
    """(NF,NF,NF) cost of every fingering of key trigram a,b,c (inf = invalid)."""
    return tri_many(model, tabs, [a], [b], [c])[0]


def bi_many(tabs, A, B):
    """(N,NF,NF) bigram fingering costs; repeats forced to one finger."""
    A, B = np.asarray(A, int), np.asarray(B, int)
    F = _FS
    t = tabs["T2"][A[:, None, None], F[None, :, None], B[:, None, None], F[None, None, :]]
    bad = (A == B)[:, None, None] & (F[:, None] != F[None, :])[None]
    return np.where(bad, np.inf, t)


def trigram_costs(model, tabs, k1, k2, k3):
    """All fingerings of a key trigram -> sorted list (cost, 'XYZ')."""
    t = tri_block(model, tabs, KI[k1], KI[k2], KI[k3])
    rows = [(float(t[f1, f2, f3]), FINGERS[f1] + FINGERS[f2] + FINGERS[f3])
            for f1, f2, f3 in itertools.product(range(NF), repeat=3) if np.isfinite(t[f1, f2, f3])]
    rows.sort()
    return rows


def best_trigram(model, tabs, k1, k2, k3, topn=3):
    return trigram_costs(model, tabs, k1, k2, k3)[:topn]


def pair_breakdown(model, tabs, k1, f1, k2, f2):
    """Per-component cost of one fingered bigram (for explanations)."""
    a, b = KI[k1], KI[k2]
    fa, fb = FI[f1], FI[f2]
    out = {n: float(v[a, fa, b, fb]) for n, v in tabs["parts"].items()}
    out["key_a"] = float(tabs["E"][a, fa])
    out["key_b"] = float(tabs["E"][b, fb])
    out["total"] = float(tabs["T2"][a, fa, b, fb])
    out["g"] = float(PF["g"][a, fa, b, fb])
    out["s"] = float(PF["s"][a, fa, b, fb])
    return out


def PF_S(a, fa, b, fb):
    """key separation s of a same-hand pair (key a with finger fa, key b with finger fb)."""
    return float(PF["s"][KI[a], FI[fa], KI[b], FI[fb]])
