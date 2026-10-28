"""
run_search.py — Run annealing for several punctuation dead-key configs.
Usage: python3 run_search.py [n_restarts] [iters] [batch_tag]
"""
import sys, time, json
sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer import Corpus
from search import Evaluator, LETTERS
from fingers import KEY_ORDER
from scorer import get_tables

CORPUS = "/home/akai/git/layout-opencode-investigation/corpus/"

CONFIGS = {
    # char -> key name holding it
    "cfgD_slashQ":  {",": ",", ".": ".", ";": ";", "/": "Q"},   # slash on ring corner
    "cfgE_semiQ":   {",": ",", ".": ".", ";": "Q", "/": "/"},   # semi on ring corner
    "cfgF_aposQ":   {",": ",", ".": ".", ";": ";", "'": "Q"},   # apostrophe in grid (slash off-grid)
    "cfgG_semiP":   {",": ",", ".": ".", ";": "P", "/": "/"},   # semi top-right (Colemak/Canary style)
}

# warm-start sources: batch1 winners + reference layouts (letters only)
BATCH1 = {
    "cfgE_semiQ": ";nrlvkuiaxhpgtmyecbsjzfdwqo,./",
    "cfgD_slashQ": "/nrlvkuiaxspctmyegb;zjfdwqo,.h",
    "cfgF_aposQ": "'nrlvkuiaxspctmyegb;zjfdwqo,.h",
}
REFS = {
    "canary":  "wlypkzxou;" "crstbfneia" "jvdgqmh/,.",
    "gallium": "bldcvjfou," "nrtsgyhaei" "xqmwzkp';.",
    "cdh":     "qwfpbjluy;" "arstgmneio" "zxcdvkh,./",
    "sturdy":  "vmlcpxfouj" "srtdy.naei" "zkqgwbh';,",
    "semimak": "flhvzqwuoy" "srntkcdeai" "x'bmjpg/,/",
}


def slots_for(dead):
    return [k for k in KEY_ORDER if k not in dead.values()]


def perm_from_layout(layout30, dead):
    """Build a letter->slot perm matching `dead`'s slot set from a reference
    layout string. Letters sitting on newly-dead keys are moved to keys that
    were dead in the reference (paired by letter frequency order)."""
    slots = slots_for(dead)
    slot_idx = {k: i for i, k in enumerate(slots)}
    # frequency order of letters (ascending: rare letters get displaced first
    # onto the worst freed keys; freed keys sorted by effort-ish col/row)
    BIG, _, _, _ = get_tables()
    # use corpus-free heuristic: letter rank by English frequency string
    freq = "etaoinshrdlcumwfgypbvkjxqz"
    rank = {c: i for i, c in enumerate(freq)}

    ref_by_key = {}
    for i, ch in enumerate(layout30):
        ref_by_key[KEY_ORDER[i]] = ch

    dead_keys = set(dead.values())
    ref_dead_keys = {k for k, ch in ref_by_key.items() if ch not in LETTERS}

    keep, displaced = {}, []
    freed = []
    for k in slots:
        ch = ref_by_key.get(k)
        if ch in LETTERS:
            keep[ch] = k
        else:
            freed.append(k)
    for k, ch in ref_by_key.items():
        if ch in LETTERS and k in dead_keys:
            displaced.append(ch)
    # freed keys are those free in new but dead/lettered... freed may be fewer
    # than displaced only if reference had letters on keys free now; rebuild:
    all_ref_letters = [ch for ch in LETTERS if ch not in displaced]
    used = set(keep.values())
    extra_free = [k for k in KEY_ORDER if k not in dead_keys and k not in used]
    displaced.sort(key=lambda c: -rank.get(c, 26))   # rarest first -> worst key
    extra_free.sort(key=lambda k: (KEY_ORDER.index(k) // 10,
                                   KEY_ORDER.index(k) % 10 > 4,
                                   KEY_ORDER.index(k) % 10))  # rough outer-first
    for ch, k in zip(displaced, extra_free):
        keep[ch] = k
    perm = [0] * 26
    for ch, k in keep.items():
        perm[ord(ch) - 97] = slot_idx[k]
    return tuple(perm)


def main():
    n_restarts = int(sys.argv[1]) if len(sys.argv) > 1 else 10
    iters = int(sys.argv[2]) if len(sys.argv) > 2 else 100000
    tag = sys.argv[3] if len(sys.argv) > 3 else "batch2"
    corpus = Corpus.load(CORPUS)
    results = {}
    for name, dead in CONFIGS.items():
        print(f"\n=== {name} dead={dead} ===", flush=True)
        ev = Evaluator(corpus, dead)
        best_all = (1e9, None)
        seeds = []
        if name in BATCH1:
            seeds.append(perm_from_layout(BATCH1[name], dead))
        for rname, rlay in REFS.items():
            seeds.append(perm_from_layout(rlay, dead))
        for r in range(n_restarts):
            seed_perm = seeds[r] if r < len(seeds) else None
            t = time.time()
            best, s = ev.anneal(seed_perm=seed_perm, iters=iters,
                                seed=hash(name) % 10000 + r * 7)
            layout = ev.layout_str(best)
            dt = time.time() - t
            print(f"  restart {r:2d}: {s:.4f} ({dt:.0f}s) {layout}"
                  + ("  [warm]" if seed_perm is not None else ""), flush=True)
            if s < best_all[0]:
                best_all = (s, layout)
        results[name] = best_all
        print(f"  BEST {name}: {best_all[0]:.4f} {best_all[1]}", flush=True)
    print("\n=== summary ===", flush=True)
    for name, (s, layout) in sorted(results.items(), key=lambda kv: kv[1][0]):
        print(f"{s:.4f}  {name:14s} {layout}", flush=True)
    json.dump({k: {"score": v[0], "layout": v[1]} for k, v in results.items()},
              open(f"/home/akai/git/layout-opencode-investigation/results/search_{tag}.json", "w"),
              indent=1)


if __name__ == "__main__":
    main()
