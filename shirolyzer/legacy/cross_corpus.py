"""
cross_corpus.py — Score layouts on the alternate corpora (robustness check).
"""
import sys, json
sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
from scorer import Corpus
from search import Evaluator, LETTERS
from fingers import KEY_ORDER

BASE = "/home/akai/git/layout-opencode-investigation/corpus/"

VARIANTS = {
    "bigtxt": ("letter_freq.json", "bigram_freq.json", "trigram_freq.json"),
    "opensub": ("opensubtitles_50k_letter_freq.json",
                "opensubtitles_50k_bigram_freq.json",
                "opensubtitles_50k_trigram_freq.json"),
    "gutenberg": ("gutenberg_letter_freq.json",
                  "gutenberg_bigram_freq.json",
                  "gutenberg_trigram_freq.json"),
}


def load_variant(name):
    lf, bf, tf = VARIANTS[name]
    L = json.load(open(BASE + lf))
    B = json.load(open(BASE + bf))
    T = json.load(open(BASE + tf))
    return Corpus(L, B, T)


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
    slot_pos = {k: i for i, k in enumerate(ev.slots)}
    return tuple(slot_pos[KEY_ORDER[layout.index(ch)]] for ch in LETTERS)


def main():
    layouts = {}
    p = "/home/akai/git/layout-opencode-investigation/results/search_batch2.json"
    try:
        layouts.update({k: v["layout"] for k, v in json.load(open(p)).items()})
    except FileNotFoundError:
        pass
    p1 = "/home/akai/git/layout-opencode-investigation/results/search_batch1.json"
    try:
        layouts.update({f"b1_{k}": v["layout"] for k, v in json.load(open(p1)).items()})
    except FileNotFoundError:
        pass
    for ref in ["Canary", "Gallium", "Colemak-DH"]:
        from compare import REF
        layouts[ref] = REF[ref]

    corpora = {n: load_variant(n) for n in VARIANTS}
    print(f"{'layout':22s} " + " ".join(f"{n:>10s}" for n in VARIANTS))
    rows = {}
    for name, lay in layouts.items():
        if len(lay) != 30:
            continue
        scores = []
        dead = dead_of(lay)
        for cname, corp in corpora.items():
            ev = Evaluator(corp, dead)
            try:
                perm = slots_of(ev, lay)
            except ValueError:
                scores.append(float("nan"))
                continue
            scores.append(ev.score(perm))
        rows[name] = scores
        print(f"{name:22s} " + " ".join(f"{s:10.4f}" for s in scores))
    # ranking stability
    print("\nrank per corpus (1=best):")
    for i, cname in enumerate(VARIANTS):
        order = sorted(rows.items(), key=lambda kv: kv[1][i])
        print(f"  {cname}: " + " > ".join(f"{k}({v[i]:.3f})" for k, v in order[:6]))


if __name__ == "__main__":
    main()
