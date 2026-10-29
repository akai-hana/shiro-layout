#!/usr/bin/env python3
"""Generate a layout with the calibrated pattern model.

The primary corpus/corpora drive the quench and anneal loops.  Secondary
corpora are optimized only after primary convergence, and only inside the
configured primary-sacrifice budget.
"""
import argparse
import json
import math
import random
import time
from pathlib import Path

import numpy as np

import comfort_geo as cg
import scorer_geo as SG

LETTERS = "abcdefghijklmnopqrstuvwxyz"
FID2 = {0: "P", 1: "R", 2: "M", 3: "I", 4: "T"}
FORDER = {"P": 0, "R": 1, "M": 2, "I": 3}
FLAT_SHARE = {"P": 0.18, "R": 0.25, "M": 0.28, "I": 0.29}
W_BALANCE = 0.4
W_HAND = 0.5
DEFAULT_LAYOUT = ";tspbxgoiwndhrlyecuafkvmzjq,./"
DEFAULT_SEEDS = (
    ("shai_best", DEFAULT_LAYOUT),
    ("cfgE", ";aiojvlnrwhfpeygtcmsbxquzkd,./"),
    ("chat_best", ";thskxpoiyrdgnwfeucalzmbvqj,./"),
    ("qwerty", "qwertyuiopasdfghjkl;zxcvbnm,./"),
    ("colemak", "qwfpgjluy;arstdhneiozxcvbkm,./"),
    ("colemak_dh", "qwfpbjluy;arstgmneiozxcdvkh,./"),
    ("canary", "wlypkzxou;crstbfneiajvdgqmh/,./"),
    ("sturdy", "vmlcpxfoujsrtdy.naeizkqgwbh';,"),
    ("gallium", "bldcvjfou,nrtsgyhaeixqmwzkp';."),
    ("semimak", "flhvzqwuoysrntkcdeai x'bmjpg/,/".replace(" ", "")),
)


def dead_of(layout):
    dead = {}
    extra = 0
    for i, ch in enumerate(layout):
        if ch not in LETTERS:
            key = ch
            if ch in dead:
                extra += 1
                key = f"\x01{extra}"
            dead[key] = cg.KEYS[i]
    return dead


def slots_of(evaluator, layout):
    pos = {k: i for i, k in enumerate(evaluator.slots)}
    return tuple(pos[cg.KEYS[layout.index(ch)]] for ch in LETTERS)


class Evaluator:
    def __init__(self, corpus, dead_keys, tables=None):
        self.corpus = corpus
        self.dead = dict(dead_keys)
        self.slots = [k for k in cg.KEYS if k not in self.dead.values()]
        if len(self.slots) != 26:
            raise SystemExit(f"need 26 letter slots, got {len(self.slots)}")
        big, tri, _, _ = tables or SG.get_tables()
        self.BIG, self.TRI = big, tri
        self.tri_ids, self.tri_counts = [], []
        for tg, cnt in corpus.T.items():
            if len(tg) == 3 and all(c in LETTERS for c in tg):
                self.tri_ids.append([ord(c) - 97 for c in tg])
                self.tri_counts.append(cnt)
        self.tri_ids = np.array(self.tri_ids, dtype=np.int32)
        self.tri_counts = np.array(self.tri_counts, dtype=np.float64)
        self.n_tri = self.tri_counts.sum() or 1.0
        self.lfreq = np.array([corpus.L.get(ch, 0.0) for ch in LETTERS], dtype=np.float64)
        self.lfreq /= self.lfreq.sum() or 1.0
        self._slot_key = np.array([cg.KI[k] for k in self.slots], dtype=np.int32)
        self._default_finger = np.array(
            [FORDER[cg.DEFAULT_FINGER[k]] for k in self.slots], dtype=np.int32)
        self._hand = np.array([int(cg.H[cg.KI[k]]) for k in self.slots], dtype=np.int32)
        tri = self.TRI.ravel()
        a = self.tri_ids[:, 0]
        b = self.tri_ids[:, 1]
        c = self.tri_ids[:, 2]
        self._pref_a = self.tri_counts * a
        self._pref_b = self.tri_counts * b
        self._pref_c = self.tri_counts * c
        self._pref_const = 0

    def _key_ids(self, perm):
        return self._slot_key[np.asarray(perm, dtype=np.int32)]

    def score(self, perm):
        kid = self._key_ids(perm)
        flat = (kid[self.tri_ids[:, 0]] * 961
                + kid[self.tri_ids[:, 1]] * 31
                + kid[self.tri_ids[:, 2]])
        cost = float((self.TRI.ravel()[flat] * self.tri_counts).sum() / self.n_tri)
        usage = np.zeros((2, 4))
        for li in range(26):
            usage[self._hand[li], self._default_finger[li]] += self.lfreq[li]
        pen = 0.0
        for h in range(2):
            total = usage[h].sum()
            if total <= 0:
                continue
            for fi, f in enumerate("PRMI"):
                share = usage[h, fi] / total
                pen += ((share - FLAT_SHARE[f]) / FLAT_SHARE[f]) ** 2
        pen *= W_BALANCE
        hl, hr = usage[0].sum(), usage[1].sum()
        pen += ((hl - hr) / max(hl + hr, 1e-9)) ** 2 * W_HAND
        return cost + pen

    def anneal(self, seed_perm=None, iters=60000, t0=0.35, t1=0.002, seed=0):
        rng = random.Random(seed)
        perm = list(seed_perm) if seed_perm else list(range(26))
        if seed_perm is None:
            rng.shuffle(perm)
        best, cur = perm[:], self.score(perm)
        best_s = cur
        for it in range(iters):
            t = t0 * (t1 / t0) ** (it / iters)
            i, j = rng.sample(range(26), 2)
            perm[i], perm[j] = perm[j], perm[i]
            s = self.score(perm)
            if s < cur or rng.random() < math.exp((cur - s) / t):
                cur = s
                if s < best_s:
                    best_s, best = s, perm[:]
            else:
                perm[i], perm[j] = perm[j], perm[i]
        return best, best_s

    def layout_str(self, perm):
        arr = [None] * 30
        for ch, kname in self.dead.items():
            arr[cg.KI[kname]] = ch
        for li, slot_idx in enumerate(perm):
            arr[cg.KI[self.slots[slot_idx]]] = LETTERS[li]
        return "".join(arr)


def quench(ev, perm, cur_s, rng):
    def improve(moves, apply, undo):
        nonlocal perm, cur_s
        changed = True
        while changed:
            changed = False
            order = list(moves)
            rng.shuffle(order)
            for move in order:
                state = apply(perm, move)
                s = ev.score(perm)
                if s < cur_s - 1e-12:
                    cur_s = s
                    changed = True
                else:
                    undo(perm, move, state)
        return perm, cur_s

    def swap(p, m):
        i, j = m
        p[i], p[j] = p[j], p[i]

    def swap_undo(p, m, _):
        swap(p, m)

    pairs = [(i, j) for i in range(26) for j in range(i + 1, 26)]
    perm, cur_s = improve(pairs, swap, swap_undo)
    two = []
    for x in range(len(pairs)):
        i1, j1 = pairs[x]
        for y in range(x + 1, len(pairs)):
            i2, j2 = pairs[y]
            if len({i1, j1, i2, j2}) == 4:
                two.append((i1, j1, i2, j2))
    rng.shuffle(two)

    def swap2(p, m):
        i1, j1, i2, j2 = m
        p[i1], p[j1] = p[j1], p[i1]
        p[i2], p[j2] = p[j2], p[i2]

    def swap2_undo(p, m, _):
        swap2(p, m)
    perm, cur_s = improve(two, swap2, swap2_undo)
    three = [(i, j, k) for i in range(26) for j in range(i + 1, 26) for k in range(j + 1, 26)]

    def cycle3(p, m):
        i, j, k = m
        state = (p[i], p[j], p[k])
        p[i], p[j], p[k] = state[1], state[2], state[0]
        return state

    def cycle3_undo(p, m, state):
        i, j, k = m
        p[i], p[j], p[k] = state
    return improve(three, cycle3, cycle3_undo)


def perturb(perm, rng, k):
    cand = perm[:]
    idx = list(range(26))
    rng.shuffle(idx)
    moved = idx[:2 * k]
    for a in range(0, len(moved) - 1, 2):
        i, j = moved[a], moved[a + 1]
        cand[i], cand[j] = cand[j], cand[i]
    return cand


def parse_budget(value):
    if value is None:
        return None
    value = str(value).strip().lower()
    try:
        if value.endswith("ms"):
            return float(value[:-2]) / 1000
        if value.endswith("s"):
            return float(value[:-1])
        if value.endswith("m"):
            return float(value[:-1]) * 60
        if value.endswith("h"):
            return float(value[:-1]) * 3600
        return float(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"invalid time budget: {value}")


class BudgetExceeded(Exception):
    pass


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--layout", action="append", default=[],
                    help="additional starting layout (repeatable)")
    ap.add_argument("--primary-corpus", action="append", default=[])
    ap.add_argument("--secondary-corpus", action="append", default=[])
    ap.add_argument("--primary-sacrifice", type=float, default=0.00023,
                    help="percent of the best primary score allowed for secondary tuning")
    ap.add_argument("--runs", type=int, default=24)
    ap.add_argument("--iters", type=int, default=60000)
    ap.add_argument("--ils", type=int, default=6)
    ap.add_argument("--seed", type=int)
    ap.add_argument("--time-budget", type=parse_budget)
    ap.add_argument("--out", default="shirolyzer/results/compute.txt")
    ap.add_argument("--out-json", default="shirolyzer/results/compute.json")
    args = ap.parse_args()

    started = time.time()
    if args.seed is None:
        args.seed = random.SystemRandom().randint(0, 2**31 - 1)
    print(f"seed={args.seed}", flush=True)
    rng = random.Random(args.seed)
    primary_paths = args.primary_corpus or ["corpora/shai/parsed/"]
    secondary_paths = args.secondary_corpus or ["corpora/spanish/parsed/"]
    primary_corpora = [SG.Corpus.load(p) for p in primary_paths]
    primary_corpus = primary_corpora[0]
    if len(primary_corpora) > 1:
        from collections import Counter
        L, B, T = Counter(), Counter(), Counter()
        for c in primary_corpora:
            L.update(c.L); B.update(c.B); T.update(c.T)
        primary_corpus = SG.Corpus(L, B, T)
    secondary_corpus = SG.Corpus.load(secondary_paths[0]) if secondary_paths else None
    tables = SG.get_tables()
    base_layout = args.layout[0] if args.layout else DEFAULT_LAYOUT
    dead = dead_of(base_layout)
    primary_ev = Evaluator(primary_corpus, dead, tables)
    secondary_ev = Evaluator(secondary_corpus, dead, tables) if secondary_corpus else None

    seeds = []
    seen = set()
    for name, lay in DEFAULT_SEEDS:
        if len(lay) == 30 and lay not in seen:
            seen.add(lay)
            seeds.append((name, list(slots_of(primary_ev, lay))))
    for i, lay in enumerate(args.layout):
        if len(lay) == 30 and lay not in seen:
            seen.add(lay)
            seeds.append((f"user{i}", list(slots_of(primary_ev, lay))))
    this_run = time.time()
    for s in range(args.runs):
        perm = list(range(26))
        random.Random(args.seed + 1000 + s).shuffle(perm)
        seeds.append((f"rand{args.seed + 1000 + s}", perm))

    results = []
    best_primary = None
    print(f"{'start':18s} {'final':>9s} {'primary':>9s}  layout", flush=True)
    for name, start in seeds:
        if args.time_budget and time.time() - started >= args.time_budget:
            print("time budget reached; saving best so far", flush=True)
            break
        perm = start[:]
        cs = primary_ev.score(perm)
        try:
            perm, cs = quench(primary_ev, perm, cs, rng)
            perm, cs = primary_ev.anneal(perm, iters=args.iters, seed=args.seed + 17)
            perm, cs = quench(primary_ev, perm, cs, rng)
        except Exception:
            raise
        lay = primary_ev.layout_str(perm)
        pri = primary_ev.score(perm)
        results.append({"start": name, "final": cs, "primary": pri,
                        "secondary": None, "layout": lay})
        if best_primary is None or pri < best_primary["primary"]:
            best_primary = results[-1]
        print(f"{name:18s} {cs:9.4f} {pri:9.4f}  {lay}", flush=True)

    if not results:
        raise SystemExit("no completed runs")

    uniq = {}
    for r in results:
        if r["layout"] not in uniq or r["final"] < uniq[r["layout"]]["final"]:
            uniq[r["layout"]] = r
    top = sorted(uniq.values(), key=lambda r: (r["primary"], r["final"]))[:4]
    print("\nILS (primary):", flush=True)
    for rank, r in enumerate(top, 1):
        if args.time_budget and time.time() - started >= args.time_budget:
            print("time budget reached during ILS; saving best so far", flush=True)
            break
        best = list(slots_of(primary_ev, r["layout"]))
        best_s = r["final"]
        for it in range(max(1, args.ils)):
            if args.time_budget and time.time() - started >= args.time_budget:
                break
            cand = perturb(best, rng, rng.choice([4, 6, 8]))
            cand, cs = quench(primary_ev, cand, primary_ev.score(cand), rng)
            cand, cs = primary_ev.anneal(cand, iters=max(1, args.iters // 2), seed=51 + it)
            cand, cs = quench(primary_ev, cand, cs, rng)
            if cs < best_s - 1e-12:
                best, best_s = cand[:], cs
        ilay = primary_ev.layout_str(best)
        ipri = primary_ev.score(best)
        results.append({"start": f"ILS{rank}:{r['start']}", "final": best_s,
                        "primary": ipri, "secondary": None, "layout": ilay})
        if ipri < best_primary["primary"]:
            best_primary = results[-1]
        print(f"  ILS{rank} {r['start']:16s} {r['final']:.4f} -> {best_s:.4f} "
              f"primary={ipri:.4f}  {ilay}", flush=True)

    final = best_primary
    if secondary_ev is not None:
        budget = abs(best_primary["primary"]) * args.primary_sacrifice / 100.0
        limit = best_primary["primary"] + budget + 1e-12

        def constrained(perm):
            pri = primary_ev.score(perm)
            if pri > limit:
                return float("inf")
            return secondary_ev.score(perm)

        candidates = sorted(uniq.values(), key=lambda r: (r["primary"], r["final"]))[:4]
        print(f"\nsecondary phase (sacrifice {args.primary_sacrifice:g}% = {budget:.3g}):", flush=True)
        best_secondary = None
        for r in candidates:
            if args.time_budget and time.time() - started >= args.time_budget:
                print("time budget reached before secondary phase", flush=True)
                break
            perm = list(slots_of(primary_ev, r["layout"]))
            cur = constrained(perm)
            if not math.isfinite(cur):
                continue
            for it in range(max(1, args.ils)):
                if args.time_budget and time.time() - started >= args.time_budget:
                    break
                cand = perturb(perm, rng, rng.choice([2, 4]))
                c = constrained(cand)
                if math.isfinite(c) and c < cur - 1e-12:
                    perm, cur = cand, c
            lay = primary_ev.layout_str(perm)
            pri = primary_ev.score(perm)
            sec = secondary_ev.score(perm)
            row = {"start": f"secondary:{r['start']}", "final": sec,
                   "primary": pri, "secondary": sec, "layout": lay}
            results.append(row)
            if best_secondary is None or sec < best_secondary["secondary"]:
                best_secondary = row
            print(f"  {r['start']:16s} primary={pri:.6f} secondary={sec:.6f}  {lay}", flush=True)
        if best_secondary is not None:
            final = best_secondary

    results.sort(key=lambda r: (r["primary"], r["final"]))
    out_lines = [
        f"seed={args.seed}",
        f"primary={','.join(primary_paths)} secondary={','.join(secondary_paths)}",
        f"primary_sacrifice_pct={args.primary_sacrifice:g}",
        f"best_primary: primary={best_primary['primary']:.9f} {best_primary['layout']}",
        f"selected: primary={final['primary']:.9f} "
        f"secondary={final['secondary'] if final['secondary'] is not None else float('nan'):.9f} "
        f"{final['layout']}",
        "",
        f"{'start':24s} {'primary':>10s} {'secondary':>10s}  layout",
    ]
    for r in results:
        sec = r["secondary"]
        out_lines.append(f"{r['start']:24s} {r['primary']:10.6f} "
                         f"{(sec if sec is not None else float('nan')):10.6f}  {r['layout']}")
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(args.out).write_text("\n".join(out_lines) + "\n")
    Path(args.out_json).write_text(json.dumps({
        "seed": args.seed,
        "primary_corpora": primary_paths,
        "secondary_corpora": secondary_paths,
        "primary_sacrifice_pct": args.primary_sacrifice,
        "elapsed_seconds": time.time() - started,
        "runs": args.runs, "iters": args.iters, "ils": args.ils,
        "results": results,
        "best_primary": best_primary,
        "selected": final,
    }, indent=1) + "\n")
    print(f"\nbest_primary: {best_primary['layout']} ({best_primary['primary']:.6f})")
    print(f"selected: {final['layout']}")
    print(f"wrote {args.out} and {args.out_json} ({time.time() - started:.0f}s)")


if __name__ == "__main__":
    main()
