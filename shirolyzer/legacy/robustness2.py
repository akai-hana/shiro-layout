#!/usr/bin/env python3
"""Round-6: full robustness sweep under the updated scorer2 model.

Scorer2-model counterpart of robustness.py (which sweeps the OLD scorer).
For each weight perturbation: patch comfort/search2 constants, reset
scorer2 table caches, rebuild, then
  - score cfgE + all community refs statically,
  - quench cfgE (1-swap, 2-swap, 3-cycle first-improvement),
  - anneal from cfgE (iters via argv, default 20k).

Baseline must reproduce cfgE's current scorer2 score before any perturbation.
"""
import json
import random
import sys
import time
from collections import defaultdict

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
import comfort
import search2
import scorer2
from scorer2 import Corpus, get_tables
from search2 import Evaluator, LETTERS
from compare import dead_of, slots_of, score_of, REF

CFG_E = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
GRAPHITE = "bldwz'foujnrtsgyhaeiqxmcvkp.-/"
ANNEAL_BEST = ";stckzdaiwnlprhyeguobfvmjqx,./"
NEW_BEST = ";stpbqdaiynhcrmkeguowvflzxj,./"
LAYOUTS = {
    "cfgE": CFG_E,
    "cfgG": "faiojwlrn;hgpeyvtcmsbqxuzkd,./",
    "annealBEST": ANNEAL_BEST,
    "NEW_BEST": NEW_BEST,
    "Sturdy": REF["Sturdy"],
    "Graphite": GRAPHITE,
    "Gallium": REF["Gallium"],
    "Colemak-DH": REF["Colemak-DH"],
    "Canary": REF["Canary"],
    "Semimak": REF["Semimak"],
    "QWERTY": REF["QWERTY"],
}
OUT_TXT = "/home/akai/git/layout-opencode-investigation/results/robustness2.txt"
OUT_JSON = "/home/akai/git/layout-opencode-investigation/results/robustness2.json"

# Each group: list of (module, [attrs]). scorer2 holds its OWN copies of the
# bigram/trigram weights (from comfort import ...), so weight groups must
# patch scorer2 (where the tables read them); SCISSOR_* are read live inside
# comfort.scissor_cost, W_FULLSPAN inside scorer2.
GROUPS = {
    "sfb": [("scorer2", ["W_SFB_SAME_ROW", "W_SFB_1ROW",
                         "W_SFB_1ROW_DOWN", "W_SFB_2ROW"])],
    "rolls": [("scorer2", ["W_ROLL_IN", "W_ROLL_OUT",
                           "W_ROLL_MI_SYM", "W_ROLL_RING_PINKY",
                           "W_ROLL_PINKY_RING",
                           "W_ROLL_LEAP_GAP2", "W_ROLL_LEAP_GAP3",
                           "W_ROLL_WIDE_GAP"])],
    "alt": [("scorer2", ["W_ALT"])],
    "redirect": [("scorer2", ["W_REDIRECT", "W_REDIRECT_WEAK"])],
    "sfs": [("scorer2", ["W_SFS_1U", "W_SFS_FAR"])],
    "continue": [("scorer2", ["W_CONTINUE"])],
    "balance": [("search2", ["W_BALANCE"])],
    "hand": [("search2", ["W_HAND"])],
    "scissors": [("comfort", ["SCISSOR_MI_HSB", "SCISSOR_MI_FSB",
                              "SCISSOR_MR_RP", "SCISSOR_MR_FSB"]),
                 ("scorer2", ["W_FULLSPAN_SCISSOR_STRAIN"])],
    "stagger": [("comfort", ["W_STAGGER_FAVOR"])],
}

SCENARIOS = [
    ("baseline", {}),
    ("sfb x1.5", {"sfb": 1.5}),
    ("sfb x0.7", {"sfb": 0.7}),
    ("rolls x1.5", {"rolls": 1.5}),
    ("rolls x0.5", {"rolls": 0.5}),
    ("alt off", {"alt": 0.0}),
    ("alt x2", {"alt": 2.0}),
    ("redirect x2", {"redirect": 2.0}),
    ("redirect x0.5", {"redirect": 0.5}),
    ("sfs x2", {"sfs": 2.0}),
    ("sfs off", {"sfs": 0.0}),
    ("balance off", {"balance": 0.0}),
    ("balance x3", {"balance": 3.0}),
    ("hand off", {"hand": 0.0}),
    ("scissors x2", {"scissors": 2.0}),
    ("scissors off", {"scissors": 0.0}),
    ("pessimistic joint", {"sfb": 1.5, "rolls": 0.6, "alt": 0.5, "sfs": 1.5,
                           "redirect": 1.5, "balance": 1.5, "scissors": 1.5,
                           "continue": 1.5}),
    ("optimistic joint", {"sfb": 0.7, "rolls": 1.4, "alt": 2.0, "sfs": 0.5,
                          "redirect": 0.5, "scissors": 0.5}),
]


def pristine():
    snap = {}
    for gname, targets in GROUPS.items():
        snap[gname] = {}
        for mod, attrs in targets:
            m = sys.modules[mod]
            snap[gname][mod] = {a: getattr(m, a) for a in attrs}
    return snap


def apply_scenario(snap, mults):
    for gname, targets in GROUPS.items():
        mult = mults.get(gname, 1.0)
        for mod, attrs in targets:
            m = sys.modules[mod]
            for a in attrs:
                setattr(m, a, snap[gname][mod][a] * mult)
    scorer2.TABLES = None
    scorer2.DEFAULT_TABLES = None


# Evaluator snapshots tables at __init__, so one per (layout, weight state).
# Cache must be cleared whenever weights change; defaultdict's default_factory
# is how the clear() gets re-armed after a .clear() (dict.clear keeps the
# factory, but a plain {} cache would be refilled before get_tables() ran).
_EV_CACHE = defaultdict(lambda: None)


def get_ev(corpus, lay):
    if _EV_CACHE[lay] is None:
        _EV_CACHE[lay] = Evaluator(corpus, dead_of(lay))
    return _EV_CACHE[lay]


def reset_evs():
    _EV_CACHE.clear()
    scorer2.TABLES = None
    scorer2.DEFAULT_TABLES = None
    get_tables()  # rebuild NOW, under the weights just applied


def quench(ev, perm, cur_s, rng):
    n = 26

    def first_improve(perm, cur_s, pairs):
        improved = True
        while improved:
            improved = False
            order = list(pairs)
            rng.shuffle(order)
            for move in order:
                i, j = move[0], move[1]
                perm[i], perm[j] = perm[j], perm[i]
                s = ev.score(tuple(perm))
                if s < cur_s - 1e-12:
                    cur_s = s
                    improved = True
                else:
                    perm[i], perm[j] = perm[j], perm[i]
        return perm, cur_s

    perm, cur_s = first_improve(perm, cur_s,
                                [(i, j) for i in range(n) for j in range(i + 1, n)])
    prs = [(i, j) for i in range(n) for j in range(i + 1, n)]
    two = []
    for a in range(len(prs)):
        i1, j1 = prs[a]
        for b in range(a + 1, len(prs)):
            i2, j2 = prs[b]
            if len({i1, j1, i2, j2}) == 4:
                two.append((i1, j1, i2, j2))
    rng.shuffle(two)
    improved = True
    while improved:
        improved = False
        for (i1, j1, i2, j2) in two:
            perm[i1], perm[j1] = perm[j1], perm[i1]
            perm[i2], perm[j2] = perm[j2], perm[i2]
            s = ev.score(tuple(perm))
            if s < cur_s - 1e-12:
                cur_s = s
                improved = True
            else:
                perm[i1], perm[j1] = perm[j1], perm[i1]
                perm[i2], perm[j2] = perm[j2], perm[i2]
    three = [(i, j, k) for i in range(n) for j in range(i + 1, n)
             for k in range(j + 1, n)]
    rng.shuffle(three)
    improved = True
    while improved:
        improved = False
        for (i, j, k) in three:
            a, b, c = perm[i], perm[j], perm[k]
            perm[i], perm[j], perm[k] = b, c, a
            s = ev.score(tuple(perm))
            if s < cur_s - 1e-12:
                cur_s = s
                improved = True
            else:
                perm[i], perm[j], perm[k] = a, b, c
    return perm, cur_s


def main():
    iters = int(sys.argv[1]) if len(sys.argv) > 1 else 20000
    snap = pristine()
    corpus = Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")
    get_tables()

    apply_scenario(snap, {})
    reset_evs()
    ev_base = get_ev(corpus, CFG_E)
    base_perm = list(slots_of(ev_base, CFG_E))
    base_s = ev_base.score(tuple(base_perm))
    print(f"baseline cfgE = {base_s:.6f}  (anneal {iters} iters)", flush=True)

    results = []
    lines = ["R6 — scorer2 weight robustness sweep (quench + "
             f"{iters} anneal from cfgE)",
             f"baseline score {base_s:.6f}", ""]
    hdr = (f"{'scenario':20s} {'cfgE':>7s} {'bestref':>7s} {'ref':>10s} "
           f"{'quench':>7s} {'anneal':>7s}  quench->anneal layout")
    lines.append(hdr)
    print(hdr, flush=True)

    for name, mults in SCENARIOS:
        t0 = time.time()
        apply_scenario(snap, mults)
        reset_evs()
        scores = {}
        for lname, lay in LAYOUTS.items():
            ev = get_ev(corpus, lay)
            scores[lname] = score_of(ev, lay)
        cfg_s = scores["cfgE"]
        refbest = min((v, k) for k, v in scores.items() if k != "cfgE")
        ev = get_ev(corpus, CFG_E)
        perm, qs = quench(ev, base_perm[:], cfg_s, random.Random(7))
        q_layout = ev.layout_str(tuple(perm))
        aperm, ans = ev.anneal(seed_perm=perm, iters=iters, seed=11)
        a_layout = ev.layout_str(tuple(aperm))
        row = (f"{name:20s} {cfg_s:7.4f} {refbest[0]:7.4f} {refbest[1]:>10s} "
               f"{qs:7.4f} {ans:7.4f}  "
               f"{'cfgE-stable' if a_layout == CFG_E else a_layout}")
        lines.append(row)
        print(row, flush=True)
        results.append({"scenario": name, "mults": mults, "cfgE": cfg_s,
                        "refbest_val": refbest[0], "refbest": refbest[1],
                        "quench": qs, "quench_layout": q_layout,
                        "anneal": ans, "anneal_layout": a_layout,
                        "scores": scores, "secs": time.time() - t0})

    apply_scenario(snap, {})
    reset_evs()
    ev = get_ev(corpus, CFG_E)
    back = score_of(ev, CFG_E)
    assert abs(back - base_s) < 1e-9, back
    lines.append("")
    lines.append(f"restored baseline {back:.6f} OK")

    txt = "\n".join(lines) + "\n"
    with open(OUT_TXT, "w") as f:
        f.write(txt)
    json.dump(results, open(OUT_JSON, "w"), indent=1)
    print("\nwrote", OUT_TXT)


if __name__ == "__main__":
    main()
