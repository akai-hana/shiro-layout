#!/usr/bin/env python3
"""A4: weight-robustness sweep.

For each weight perturbation: patch scorer/search/fingers module constants,
reset table caches, rebuild, then
  - score cfgE + all community refs statically (same dead-key-per-layout rule
    as compare.py),
  - quench cfgE (1-swap, 2-swap, 3-cycle first-improvement, deep_search moves),
  - short anneal from cfgE (8k iters) to see if a better layout sits near it
    under the perturbed objective.

Baseline must reproduce 4.3877 before any perturbation (sanity).
"""
import json
import random
import sys
import time

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
import scorer
import search
import fingers
from scorer import Corpus, get_tables
from search import Evaluator, LETTERS
from compare import dead_of, slots_of, score_of, REF

CFG_E = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
GRAPHITE = "bldwz'foujnrtsgyhaeiqxmcvkp.-/"
LAYOUTS = {
    "cfgE": CFG_E,
    "cfgG": "faiojwlrn;hgpeyvtcmsbqxuzkd,./",
    "Sturdy": REF["Sturdy"],
    "Graphite": GRAPHITE,
    "Gallium": REF["Gallium"],
    "Colemak-DH": REF["Colemak-DH"],
    "Canary": REF["Canary"],
    "Semimak": REF["Semimak"],
    "QWERTY": REF["QWERTY"],
}
OUT_TXT = "/home/akai/git/layout-opencode-investigation/results/robustness.txt"
OUT_JSON = "/home/akai/git/layout-opencode-investigation/results/robustness.json"

GROUPS = {
    "sfb":        ("scorer", ["W_SFB_SAME_ROW", "W_SFB_1ROW", "W_SFB_2ROW"]),
    "rolls":      ("scorer", ["W_ROLL_IN", "W_ROLL_OUT", "W_ROLL_LEAP"]),
    "alt":        ("scorer", ["W_ALT"]),
    "redirect":   ("scorer", ["W_REDIRECT", "W_REDIRECT_WEAK"]),
    "sfs":        ("scorer", ["W_SFS_1U", "W_SFS_FAR"]),
    "continue":   ("scorer", ["W_CONTINUE"]),
    "balance":    ("search", ["W_BALANCE"]),
    "hand":       ("search", ["W_HAND"]),
    "scissors":   ("fingers", ["SCISSOR_MI_HSB", "SCISSOR_MI_FSB", "SCISSOR_MR_RP"]),
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
    for gname, (mod, attrs) in GROUPS.items():
        m = sys.modules[mod]
        snap[gname] = {a: getattr(m, a) for a in attrs}
    return snap


def apply_scenario(snap, mults):
    for gname, (mod, attrs) in GROUPS.items():
        m = sys.modules[mod]
        mult = mults.get(gname, 1.0)
        for a in attrs:
            setattr(m, a, snap[gname][a] * mult)
    scorer.TABLES = None
    scorer.DEFAULT_TABLES = None


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
    snap = pristine()
    corpus = Corpus.load("/home/akai/git/layout-opencode-investigation/corpus/")
    get_tables()  # baseline tables

    apply_scenario(snap, {})
    ev_base = Evaluator(corpus, dead_of(CFG_E))
    base_perm = list(slots_of(ev_base, CFG_E))
    base_s = ev_base.score(tuple(base_perm))
    assert abs(base_s - 4.387726696836196) < 1e-9, base_s

    results = []
    lines = ["A4 — weight robustness sweep (quench + 8k anneal from cfgE)",
             f"baseline score {base_s:.4f} confirmed", ""]
    hdr = (f"{'scenario':20s} {'cfgE':>7s} {'bestref':>7s} {'ref':>10s} "
           f"{'quench':>7s} {'anneal':>7s}  quench->anneal layout")
    lines.append(hdr)
    print(hdr, flush=True)

    for name, mults in SCENARIOS:
        t0 = time.time()
        apply_scenario(snap, mults)
        # static scores
        scores = {}
        for lname, lay in LAYOUTS.items():
            ev = Evaluator(corpus, dead_of(lay))
            scores[lname] = score_of(ev, lay)
        cfg_s = scores["cfgE"]
        refbest = min((v, k) for k, v in scores.items() if k != "cfgE")
        # quench cfgE
        ev = Evaluator(corpus, dead_of(CFG_E))
        perm, qs = quench(ev, base_perm[:], cfg_s, random.Random(7))
        q_layout = ev.layout_str(tuple(perm))
        # short anneal from quenched
        aperm, ans = ev.anneal(seed_perm=perm, iters=8000, seed=11)
        a_layout = ev.layout_str(tuple(aperm))
        row = (f"{name:20s} {cfg_s:7.4f} {refbest[0]:7.4f} {refbest[1]:>10s} "
               f"{qs:7.4f} {ans:7.4f}  "
               f"{'cfgE-stable' if a_layout == CFG_E else a_layout}")
        lines.append(row)
        print(row, flush=True)
        results.append({"scenario": name, "mults": mults, "cfgE": cfg_s,
                        "refbest_val": refbest[0], "refbest": refbest[1],
                        "quench": qs, "quench_layout": q_layout,
                        "anneal8k": ans, "anneal_layout": a_layout,
                        "scores": scores, "secs": time.time() - t0})

    # restore + verify
    apply_scenario(snap, {})
    ev = Evaluator(corpus, dead_of(CFG_E))
    back = score_of(ev, CFG_E)
    assert abs(back - base_s) < 1e-9, back
    lines.append("")
    lines.append(f"restored baseline {back:.4f} OK")

    txt = "\n".join(lines) + "\n"
    with open(OUT_TXT, "w") as f:
        f.write(txt)
    json.dump(results, open(OUT_JSON, "w"), indent=1)
    print("\nwrote", OUT_TXT)


if __name__ == "__main__":
    main()
