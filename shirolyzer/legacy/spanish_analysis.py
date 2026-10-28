#!/usr/bin/env python3
"""Spanish-vs-English analysis for the cfgE winner.

Answers: which letters would Spanish lock in place, and what would the
fringe look like if the English keystones+core are pinned and Spanish
alone decides the remaining 17 letters?

Parts:
  1. score winner + cfgG + references under the Spanish corpus
  2. pairwise swap sensitivity under Spanish (vs English medians)
  3. pinned annealing: letters e s a t h n r i o frozen; remaining 17
     annealed under Spanish (and under English as a control)
"""
import math
import random
import statistics

from scorer import Corpus
from search import Evaluator, LETTERS
from compare import dead_of, slots_of, score_of, REF

WIN = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
CFGG = "faiojwlrn;hgpeyvtcmsbqxuzkd,./"
PINNED = set("estah nrio".replace(" ", ""))  # EN keystones + core

corpus_en = Corpus.load("../corpus/")
corpus_es = Corpus.load("../corpus/spanish/")
ev_en = Evaluator(corpus_en, dead_of(WIN))
ev_es = Evaluator(corpus_es, dead_of(WIN))


def perm_to_layout(perm, ev, base_layout=WIN):
    """Build the 30-char layout: letters into their slots, dead keys kept."""
    lay = list(base_layout)
    slot_pos = {k: i for i, k in enumerate(ev.slots)}
    base_idx = {}
    for i, k in enumerate(KEY_ORDER):
        base_idx[k] = i
    # place each letter at the key currently assigned by perm
    for li, ch in enumerate(LETTERS):
        key = ev.slots[perm[li]]
        pos = base_idx[key]
        # find this letter's current position and remove it first
        cur = lay.index(ch)
        if cur != pos:
            # swap contents so dead keys stay put
            other = lay[pos]
            lay[pos] = ch
            lay[cur] = other
    return "".join(lay)


from fingers import KEY_ORDER  # noqa: E402  (needed by perm_to_layout)


def pinned_anneal(ev, base_perm, free_idx, iters=100000, seed=0, t0=0.35, t1=0.002):
    rng = random.Random(seed)
    perm = list(base_perm)
    best = perm[:]
    best_s = ev.score(tuple(perm))
    cur_s = best_s
    for it in range(iters):
        T = t0 * (t1 / t0) ** (it / iters)
        i, j = rng.sample(free_idx, 2)
        perm[i], perm[j] = perm[j], perm[i]
        s = ev.score(tuple(perm))
        if s < cur_s or rng.random() < math.exp((cur_s - s) / T):
            cur_s = s
            if s < best_s:
                best_s, best = s, perm[:]
        else:
            perm[i], perm[j] = perm[j], perm[i]
    return best_s, tuple(best)


def main():
    out = []

    def P(*a):
        s = " ".join(str(x) for x in a)
        print(s)
        out.append(s)

    P("=" * 78)
    P("SPANISH ANALYSIS (English remains primary; keystones pinned)")
    P("=" * 78)
    P()
    P("-- 1. layout scores under Spanish (lower = better) --")
    rows = [("WINNER (cfgE)", WIN), ("alt cfgG", CFGG)]
    rows += [(n, REF[n]) for n in
             ["Gallium", "Colemak-DH", "Canary", "Sturdy", "Semimak", "Colemak", "QWERTY"]]
    P(f"{'layout':16s} {'ES score':>8s} {'EN score':>8s}")
    win_es = score_of(ev_es, WIN)
    win_en = score_of(ev_en, WIN)
    for name, lay in rows:
        P(f"{name:16s} "
          f"{score_of(Evaluator(corpus_es, dead_of(lay)), lay):8.3f} "
          f"{score_of(Evaluator(corpus_en, dead_of(lay)), lay):8.3f}")
    P()

    P("-- 2. letter sensitivity under Spanish (median relocation cost) --")
    P(f"{'ltr':3s} {'ES%':>5s} {'EN%':>5s} {'medEN':>6s} {'medES':>6s}  tier change")
    sens = {}
    for tag, ev, corp in (("EN", ev_en, corpus_en), ("ES", ev_es, corpus_es)):
        deltas = {a: [] for a in LETTERS}
        for i, a in enumerate(LETTERS):
            for b in LETTERS[i + 1:]:
                lay = list(WIN)
                ia, ib = lay.index(a), lay.index(b)
                lay[ia], lay[ib] = lay[ib], lay[ia]
                d = ev.score(slots_of(ev, "".join(lay))) - ev.score(slots_of(ev, WIN))
                deltas[a].append(d)
                deltas[b].append(d)
        sens[tag] = {a: statistics.median(ds) for a, ds in deltas.items()}
    def tier(m):
        return "K" if m >= 1.0 else "C" if m >= 0.28 else "M" if m >= 0.13 else "F"
    for a in sorted(LETTERS, key=lambda x: -sens["EN"][x]):
        me, ms = sens["EN"][a], sens["ES"][a]
        te, ts = tier(me), tier(ms)
        note = "" if te == ts else f"{te}->{ts}"
        P(f"{a:3s} {corpus_es.L[a]/corpus_es.total_letters*100:5.2f} "
          f"{corpus_en.L[a]/corpus_en.total_letters*100:5.2f} {me:6.3f} {ms:6.3f}  {note}")
    P()

    base_perm = slots_of(ev_en, WIN)
    free_idx = [li for li, ch in enumerate(LETTERS) if ch not in PINNED]
    P(f"-- 3. pinned annealing: frozen letters {''.join(sorted(PINNED))} --")
    P(f"   free letters: {''.join(LETTERS[li] for li in free_idx)}")
    results = {}
    for tag, ev in (("EN", ev_en), ("ES", ev_es)):
        best_s, best_perm = None, None
        for seed in range(16):
            s, p = pinned_anneal(ev, base_perm, free_idx, seed=seed)
            if best_s is None or s < best_s:
                best_s, best_perm = s, p
        lay = perm_to_layout(best_perm, ev)
        results[tag] = (best_s, lay)
        P(f"   [{tag} objective] pinned-optimal fringe: {best_s:.4f}  {lay}")

    en_s, en_lay = results["EN"]
    es_s, es_lay = results["ES"]
    P()
    P("-- 4. summary --")
    P(f"   EN winner under EN: {win_en:.4f} | under ES: {win_es:.4f}")
    P(f"   EN-pinned fringe  under EN: {en_s:.4f} | under ES: "
      f"{score_of(ev_es, en_lay):.4f}")
    P(f"   ES-pinned fringe  under EN: {score_of(ev_en, es_lay):.4f} | under ES: "
      f"{es_s:.4f}")
    diff = [(a, en_lay, es_lay) for a in [0]]
    moves = []
    for i, ch in enumerate(LETTERS):
        pe, ps = en_lay.index(ch), es_lay.index(ch)
        if pe != ps:
            from fingers import KEY_ORDER as KO
            moves.append(f"{ch}: {KO[pe]}->{KO[ps]}")
    P(f"   fringe moves EN-pinned -> ES-pinned: {'; '.join(moves) if moves else 'NONE'}")

    with open("../results/spanish_analysis.txt", "w") as f:
        f.write("\n".join(out) + "\n")
    print("\nwrote results/spanish_analysis.txt")


if __name__ == "__main__":
    main()
