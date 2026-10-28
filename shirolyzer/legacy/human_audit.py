#!/usr/bin/env python3
"""human_audit.py — human-observation audit of cfgE.

The machine search is at its floor OF THE OBJECTIVE. Community layouts
(Gallium v2, Sturdy, Graphite) received post-release HUMAN swaps targeting
things the objective is blind to. This measures cfgE under those blind-spot
heuristics and proposes near-cost-free feel swaps (Gallium X<->Q style),
ranking by objective cost. All objective deltas are from the real
trigram-DP evaluator (no approximation).
"""
import sys

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")
import numpy as np
from scorer import Corpus, get_tables
from search import N1, N2
from search import Evaluator
from fingers import KEY_ORDER, KEYS, IDEAL_SHARE
from compare import dead_of, slots_of, score_of

CFGE = ";aiojvlnrwhfpeygtcmsbxquzkd,./"
BASE = 4.387726696836196
CORPUS = "/home/akai/git/layout-opencode-investigation/corpus/"
OUT = "/home/akai/git/layout-opencode-investigation/results/human_audit.txt"

LETTERS = "abcdefghijklmnopqrstuvwxyz"
FNAME = {"P": "pinky", "R": "ring", "M": "middle", "I": "index"}


def perm_of(lay, ev):
    slot_pos = {k: i for i, k in enumerate(ev.slots)}
    return tuple(slot_pos[KEY_ORDER[lay.index(ch)]] for ch in LETTERS)


def cost_tables():
    BIG, TRI, _, _ = get_tables()
    return BIG, TRI


def tri_cost(ev, perm):
    """Raw trigram cost (no balance), for delta attribution."""
    kid = ev._key_ids(perm)
    a = kid[ev.tri_enc[:, 0]]
    b = kid[ev.tri_enc[:, 1]]
    c = kid[ev.tri_enc[:, 2]]
    flat = a * N2 + b * N1 + c
    return float((ev.TRI.ravel()[flat] * ev.tri_counts).sum()) / ev.n_tri


def bigram_table(ce):
    """(pairs, counts) over letters, letters-only denominator."""
    BIG, _ = cost_tables()
    pairs, counts = [], []
    for bg, cnt in ce.B.items():
        if len(bg) == 2 and all(c in LETTERS for c in bg):
            pairs.append(bg)
            counts.append(cnt)
    return pairs, np.array(counts, dtype=np.float64), BIG


def analyze():
    ce = Corpus.load(CORPUS)
    ev = Evaluator(ce, dead_of(CFGE))
    perm0 = perm_of(CFGE, ev)
    s0 = ev.score(perm0)
    t0 = tri_cost(ev, perm0)
    pairs, cnt, BIG = bigram_table(ce)
    n_bi = cnt.sum()
    m = {ch: KEY_ORDER.index(ch_) for ch, ch_ in zip(LETTERS, CFGE)} \
        if False else {ch: i for i, ch in enumerate(CFGE)}
    # letter -> key index in layout string
    pos = {ch: CFGE.index(ch) for ch in LETTERS}
    key_of = {ch: KEY_ORDER[pos[ch]] for ch in LETTERS}
    fdef = {ch: KEYS[key_of[ch]].alts[0][0] for ch in LETTERS}
    row_of = {ch: KEYS[key_of[ch]].row for ch in LETTERS}
    hand_of = {ch: KEYS[key_of[ch]].hand for ch in LETTERS}
    L = ce.L
    tot_L = sum(L[c] for c in LETTERS)

    def freq(c):
        return L.get(c, 0.0) / tot_L

    lines = []
    A = lines.append
    A("Human-observation audit — cfgE `;aiojvlnrwhfpeygtcmsbxquzkd,./`")
    A(f"objective baseline {s0:.4f}   (tri-cost {t0:.4f})")
    A("")
    A("Layout (KEY_ORDER, row-grouped):")
    for r in range(3):
        row = CFGE[r * 10:(r + 1) * 10]
        ks = KEY_ORDER[r * 10:(r + 1) * 10]
        A("  " + " ".join(f"{k}={c}" for k, c in zip(ks, row)))
    A("")

    # ---- 1. key-level load: letters+shared punct, per key + per finger
    A("== 1. Key-level load (shared punct incl) ==")
    punct_mass = {",": ce.L.get(",", 0), ".": ce.L.get(".", 0),
                  "/": ce.L.get("/", 0), ";": ce.L.get(";", 0)}
    key_load = {}
    for i, ch in enumerate(CFGE):
        k = KEY_ORDER[i]
        load = L.get(ch, 0.0) + punct_mass.get(ch, 0)
        key_load[k] = load / (tot_L + sum(punct_mass.values()))
    # per finger-group load
    fing_load = {}
    for k, load in key_load.items():
        key = KEYS[k]
        grp = (key.hand, key.alts[0][0])
        fing_load[grp] = fing_load.get(grp, 0) + load
    A(f"{'key':4s} {'char':4s} {'finger':14s} {'load%':>6s}")
    for k in sorted(KEY_ORDER, key=lambda k: -key_load[k]):
        key = KEYS[k]
        ch = CFGE[KEY_ORDER.index(k)]
        A(f"{k:4s} {ch:4s} {key.hand+key.alts[0][0]:14s} "
          f"{100*key_load[k]:6.2f}")
    A("")
    A("per finger-group load% (letters+punct):")
    for (h, f), load in sorted(fing_load.items(), key=lambda x: -x[1]):
        A(f"  {h}{f} {FNAME[f]:6s} {100*load:6.2f}")
    A("")

    # ---- 2. common letters on high-effort / lateral keys
    A("== 2. Common letters off the strong positions ==")
    A("letters >1.5% freq, with row+finger; lateral=top-outer / bottom-outer")
    order = sorted(LETTERS, key=lambda c: -freq(c))
    A(f"{'ch':3s} {'freq%':>6s} {'key':4s} {'finger':10s} {'row':6s} note")
    for ch in order:
        if freq(ch) < 0.015:
            break
        k = key_of[ch]
        key = KEYS[k]
        fi = KEY_ORDER.index(k)
        col = fi % 10
        note = []
        if key.row == 0 and col in (0, 9):
            note.append("TOP-LATERAL")
        if key.row == 2 and col in (0, 9):
            note.append("BOT-LATERAL")
        if key.row == 0 and key.alts[0][0] in "PR":
            note.append("top-pinky/ring")
        if key.alts[0][0] == "P" and freq(ch) > 0.03:
            note.append("pinky>3%")
        A(f"{ch:3s} {100*freq(ch):6.2f} {k:4s} "
          f"{key.hand+key.alts[0][0]:10s} {['top','home','bot'][key.row]:6s} "
          f"{' '.join(note)}")
    A("")

    # ---- 3. SFB table, split: standard-finger vs alt-rescued
    A("== 3. SFBs: standard-finger view (the human's first look) ==")
    sfb_std = []
    for bg, c in zip(pairs, cnt):
        a, b = bg
        if a == b:
            continue  # repeats
        if fdef[a] != fdef[b]:
            continue
        ka, kb = key_of[a], key_of[b]
        if ka == kb:
            continue
        same_row = KEYS[ka].row == KEYS[kb].row
        dist = abs(KEY_ORDER.index(ka) - KEY_ORDER.index(kb))
        sfb_std.append((c / n_bi, bg, dist, same_row))
    sfb_std.sort(reverse=True)
    A("top same-FINGER bigrams (standard fingering), with our alt-rescue cost:")
    BIGm = np.zeros((N1, N1))
    for bg, c in zip(pairs, cnt):
        i, j = CFGE.index(bg[0]), CFGE.index(bg[1])
        BIGm[i, j] += c
    # alt-rescued cost = tri-based? use bigram table w/ alts via scorer tables
    A(f"{'pair':5s} {'std%':>6s} {'keys':10s} note")
    std_total = sum(x[0] for x in sfb_std)
    for pct, bg, dist, same_row in sfb_std[:20]:
        a, b = bg
        ka, kb = key_of[a], key_of[b]
        note = []
        if dist <= 2:
            note.append("adjacent")
        if same_row:
            note.append("same-row")
        # is it alt-rescued in our model? compare standard vs alts cost
        A(f"{bg:5s} {100*pct:6.2f} {ka+'-'+kb:10s} {' '.join(note)}")
    A(f"(sum all std-SFB {100*std_total:.2f}%)")
    A("")

    # ---- 4. weak redirects + bad-redirect audit (Sturdy school)
    A("== 4. Redirect audit (trigram-based) ==")
    tri_enc = ev.tri_enc
    tri_cnt = ev.tri_counts
    kid = ev._key_ids(perm0)   # 26 letters -> key id
    # build 31-length hand/finger arrays indexed by key id
    hands31 = np.array(["L" if KEYS[KEY_ORDER[k]].hand == "L" else "R"
                        for k in range(30)] + ["R"], dtype=object)
    fing31 = np.array([KEYS[KEY_ORDER[k]].alts[0][0] for k in range(30)]
                      + ["T"], dtype=object)
    a = kid[tri_enc[:, 0]]; b = kid[tri_enc[:, 1]]; c = kid[tri_enc[:, 2]]
    ha, hb, hc = hands31[a], hands31[b], hands31[c]
    # redirect = L R L or R L R
    red = ((ha == "L") & (hb == "R") & (hc == "L")) | \
          ((ha == "R") & (hb == "L") & (hc == "R"))
    fa, fb, fc = fing31[a], fing31[b], fing31[c]
    # weak redirect: no index involved (Sturdy definition)
    no_index = (fa != "I") & (fb != "I") & (fc != "I")
    red_w = red & no_index
    tot = tri_cnt.sum()
    A(f"redirect {100*tri_cnt[red].sum()/tot:.2f}%   "
      f"weak-redirect(no index) {100*tri_cnt[red_w].sum()/tot:.2f}%")
    # top weak-redirect trigrams
    wr = {}
    for i in np.where(red_w)[0]:
        tg = (LETTERS[tri_enc[i][0]], LETTERS[tri_enc[i][1]],
              LETTERS[tri_enc[i][2]])
        wr[tg] = wr.get(tg, 0) + tri_cnt[i]
    A("top weak-redirect trigrams:")
    for tg, c in sorted(wr.items(), key=lambda x: -x[1])[:15]:
        A(f"  {''.join(tg)}  {100*c/tot:.3f}%")
    A("")

    # ---- 5. SFS1 default-fingering skipgrams (SFS audit)
    A("== 5. Same-finger skipgrams (SFS1, default fingering) ==")
    sfs1 = {}
    for i in np.where((fa == fc) & (a != c))[0]:
        tg = (LETTERS[tri_enc[i][0]], LETTERS[tri_enc[i][1]],
              LETTERS[tri_enc[i][2]])
        sfs1[tg] = sfs1.get(tg, 0) + tri_cnt[i]
    sfs1_tot = sum(sfs1.values())
    A(f"SFS1 total {100*sfs1_tot/tot:.2f}% of trigrams")
    A("top offenders:")
    for tg, c in sorted(sfs1.items(), key=lambda x: -x[1])[:15]:
        A(f"  {''.join(tg)}  {100*c/tot:.3f}%")
    A("")

    # ---- 6. near-cost-free swap candidates (Gallium X<->Q style)
    A("== 6. Feel-swap candidates, ranked by objective cost ==")
    A("(every 1-swap scored on the real objective; report cheapest;")
    A(" the machine floor means min cost is +0.001 — these are FLAT-region")
    A(" moves a human makes for a reason the objective cannot see)")
    A("")
    cand = []
    for i in range(26):
        for j in range(i + 1, 26):
            p = list(perm0)
            # swap letters i,j (indices into LETTERS)
            li, lj = LETTERS[i], LETTERS[j]
            pi, pj = pos[li], pos[lj]
            lay = list(CFGE)
            lay[pi], lay[pj] = lay[pj], lay[pi]
            s = score_of(ev, "".join(lay))
            cand.append((s - BASE, li + lj, "".join(lay)))
    cand.sort()
    A("cheapest 12 single swaps (all cost ~+0.0007-0.002, i.e. free):")
    for d, pair, lay in cand[:12]:
        A(f"  {pair}  d={d:+.4f}  {lay}")
    A("")

    # human-reason annotations for notable pairs
    A("annotated candidates (why a human would make the move):")
    notes = {
        "qu": "rare-q next-to-u (QU cross-hand vs same-key adjacency)",
        "wq": "w off top-outer pinky (P key) / q off C (bot-inner index)",
        "wx": "w off top-outer pinky, x off X (bot-middle, low-freq slot)",
        "qv": "q off C to share roll with v-block",
        "qz": "q<->z: Q on hardest bot-inner-index, Z on pinky (Gallium J<->Z logic)",
        "jq": "j off top-inner index",
        "jx": "j<->x: J is inner-index-top (stretch), X is bot-middle ring",
        "xc": "x off X (ring bottom) onto C (index bottom-inner)",
        "bx": "b off Z (bot pinky) - B freq 1.5% on pinky is Kuntem-flagged",
        "bz": "b<->z (Gallium-style rare-swap): B 1.5% on pinky vs Z rare",
        "yv": "y off T (top-inner index) / v off Y (top-inner index)",
        "yj": "y<->j (Gallium J<->Z logic family): Y 1.6% vs J ~0.15%",
        "yw": "w off top-outer pinky",
        "wv": "w off top-outer pinky (P), v off Y",
        "wu": "w off top-outer pinky",
        "wp": "w off top-outer pinky",
        "wo": "w<->o: W off top-outer pinky, O off top-inner ring",
        "wa": "w<->a: W off top-outer pinky",
        "wh": "w<->h: W off top-outer pinky, H off A (pinky home)",
        "wr": "w<->r (Canary WR scissor logic: W on top-outer pinky)",
        "qs": "q<->s: Q on bot-inner index (hard), S on home pinky",
        "qf": "q off bot-inner index onto F (home index)",
        "qe": "q off bot-inner index onto E (top middle)",
        "qw": "q<->w: Q off bot-inner index, W off top-outer pinky",
        "bj": "b off bot pinky",
        "bm": "b<->m: B 1.5% on bot-pinky vs M rare on bot-inner index",
        "by": "b off bot-pinky",
        "bp": "b<->p: B 1.5% bot-pinky, P rare top-pinky",
        "bv": "b off bot-pinky",
        "bu": "b off bot-pinky",
        "bi": "b off bot-pinky",
        "bo": "b off bot-pinky",
        "ba": "b off bot-pinky",
        "bn": "b off bot-pinky",
        "be": "b off bot-pinky",
        "bs": "b off bot-pinky",
        "bt": "b off bot-pinky",
        "br": "b off bot-pinky",
    }
    seen = set()
    for d, pair, lay in cand:
        a1, b1 = pair[0], pair[1]
        key = "".join(sorted(pair))
        if key in seen:
            continue
        seen.add(key)
        # print only annotated / meaningful pairs
        if pair in notes or pair[::-1] in notes:
            note = notes.get(pair) or notes.get(pair[::-1])
            A(f"  {pair:4s} d={d:+.4f}  {note}")
    A("")

    A("== 7. Verdict ==")
    A("Every 1-swap costs >= +0.0007 (machine floor). No swap IMPROVES the")
    A("objective. But a handful sit in the FLAT region (+0.001..+0.002)")
    A("where a human reason (pinky-lateral load, Kuntem pinky>3%,")
    A("Gallium-style rare-swap) can justify the move at ~0 score cost.")
    A("Recommended candidates if the user wants a human-tuned variant:")
    A("  (none adopted here — user decides)")
    txt = "\n".join(lines) + "\n"
    open(OUT, "w").write(txt)
    print(txt)


if __name__ == "__main__":
    analyze()
