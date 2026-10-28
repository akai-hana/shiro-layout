#!/usr/bin/env python3
"""Audit the calibrated Shai-primary winner.

Checks:
  - composite / primary / Spanish score decomposition
  - questionnaire hard probes on physical-key sequences
  - corpus-wide impossible, SFB, SFS, and alt-gate scans
  - actual alt-finger usage under the trigram table
  - corpus sensitivity against prior winners
"""
import argparse
import itertools
import json
import sys
from collections import Counter, defaultdict

sys.path.insert(0, "/home/akai/git/layout-opencode-investigation/analysis")

import ansi
import comfort
from compare import REF, dead_of, slots_of
from fingers import KEYS, KEY_ORDER
from objective import (DEFAULT_PRIMARY_SACRIFICE, PRIMARY_PATH,
                       SECONDARY_PATH, build_composite, load_primary,
                       load_secondary)
from scorer2 import (N, Corpus, FID, _alt_context_penalty, _bigram_cost,
                     _trigram_cost, get_tables)
from search2 import Evaluator, LETTERS

ROOT = "/home/akai/git/layout-opencode-investigation/"
BEST = ";tspbxgoiwndhrlyecuafkvmzjq,./"
CFG_E = ";aiojvlnrwhfpeygtcmsbxquzkd,./"

REFS = {
    "shai_best": BEST,
    "r7_best": ";stpbqdaiynhcrmkeguowvflzxj,./",
    "r6_best": ";stckqdaiwnlprhyeguomfvbzxj,./",
    "old_shai_lead": ";trdpzgaifsmcnlxehuowkvbjqy,./",
    "old_shai_alt1": ";tncbxgaifsdprlkehuowzvmjqy,./",
    "old_shai_alt2": ";thsbxcoiwndprlyeguafkvmzqj,./",
    "chat_best": ";thskxpoiyrdgnwfeucalzmbvqj,./",
    "cfgE": CFG_E,
    "qwerty": "qwertyuiopasdfghjkl;zxcvbnm,./",
    "Graphite": "bldwz'foujnrtsgyhaeiqxmcvkp.-/",
}
_seen_refs = set(REFS.values())
for _name, _lay in REF.items():
    if len(_lay) == 30 and _lay not in _seen_refs and _name != "QWERTY":
        REFS[_name] = _lay
        _seen_refs.add(_lay)

CORPORA = {
    "shai": ROOT + "corpus/shai/",
    "spanish": ROOT + "corpus/spanish/",
}

FID2 = {v: k for k, v in FID.items()}


def eff(name, finger):
    prem = dict(KEYS[name].alts)[finger]
    return comfort.travel_cost(name, finger, KEYS[name].hand) + comfort.TRAVEL_SCALE * prem


def options(name):
    return [f for f, _ in KEYS[name].alts]


def impossible(k1, f1, k2, f2):
    return (comfort.impossible_mr(k1, f1, k2, f2)
            or comfort.impossible_ir(k1, f1, k2, f2)
            or comfort.impossible_cross(k1, f1, k2, f2)
            or comfort.impossible_rp(k1, f1, k2, f2))


def keymap(lay):
    return {ch: KEY_ORDER[i] for i, ch in enumerate(lay) if ch in LETTERS}


def score_parts(ev, perm):
    kid = ev._key_ids(perm)
    a = kid[ev.tri_enc[:, 0]]
    b = kid[ev.tri_enc[:, 1]]
    c = kid[ev.tri_enc[:, 2]]
    flat = a * (N * N) + b * N + c
    raw = float((ev.TRI.ravel()[flat] * ev.tri_counts).sum()) / ev.n_tri
    usage = {"L": [0.0] * 4, "R": [0.0] * 4}
    fix = {"P": 0, "R": 1, "M": 2, "I": 3}
    for li in range(26):
        key = KEY_ORDER[kid[li]]
        finger = KEYS[key].alts[0][0]
        usage[KEYS[key].hand][fix[finger]] += ev.lfreq[li]
    pen = 0.0
    for hand in "LR":
        hs = sum(usage[hand])
        if hs <= 0:
            continue
        for fi, finger in enumerate(["P", "R", "M", "I"]):
            share = usage[hand][fi] / hs
            ideal = comfort.FLAT_SHARE[finger]
            pen += ((share - ideal) / ideal) ** 2
    pen *= comfort.W_BALANCE_EVEN
    hl, hr = sum(usage["L"]), sum(usage["R"])
    pen += ((hl - hr) / max(hl + hr, 1e-9)) ** 2 * comfort.W_HAND_BALANCE
    return raw, pen, raw + pen, usage


def probe_rows(seq):
    rows = []
    for fs in itertools.product(*(options(k) for k in seq)):
        bad = False
        for i in range(len(seq)):
            for j in range(i + 1, len(seq)):
                if seq[i] == seq[j] and fs[i] != fs[j]:
                    bad = True
        if bad:
            continue
        c = sum(eff(k, f) for k, f in zip(seq, fs))
        for i in range(len(seq) - 1):
            c += _bigram_cost(seq[i], fs[i], seq[i + 1], fs[i + 1])
        if len(seq) == 3:
            c += _trigram_cost(seq[0], fs[0], seq[1], fs[1], seq[2], fs[2])
        for i, (k, f) in enumerate(zip(seq, fs)):
            neighbours = tuple(kk for j, kk in enumerate(seq) if j != i)
            c += _alt_context_penalty(k, f, neighbours)
        rows.append((c, fs))
    rows.sort()
    return rows


def fmt_probe(seq, limit=6):
    rows = probe_rows(seq)
    out = [f"  {''.join(seq)}:"]
    for c, fs in rows[:limit]:
        parts = []
        for k, f in zip(seq, fs):
            d = KEYS[k].alts[0][0]
            parts.append(f"{k}:{f}{'*' if f != d else ''}")
        out.append(f"    {c:8.3f}  {' -> '.join(parts)}")
    return "\n".join(out), rows


def pct(a, b):
    return 100.0 * a / b if b else 0.0


def score_layout(corpus, lay):
    ev = Evaluator(corpus, dead_of(lay))
    raw, bal, total, usage = score_parts(ev, slots_of(ev, lay))
    return {"raw": raw, "balance": bal, "total": total}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--layout", default=BEST)
    ap.add_argument("--primary", default=PRIMARY_PATH)
    ap.add_argument("--secondary", default=SECONDARY_PATH)
    ap.add_argument("--primary-sacrifice", type=float,
                    default=DEFAULT_PRIMARY_SACRIFICE)
    ap.add_argument("--out", default=ROOT + "results/round8/audit_shai.txt")
    ap.add_argument("--out-json", default=ROOT + "results/round8/audit_shai.json")
    args = ap.parse_args()
    lay = args.layout
    if len(lay) != 30:
        sys.exit("layout must be 30 chars")
    t0 = __import__("time").time()
    primary = load_primary(args.primary)
    secondary = load_secondary(args.secondary)
    composite, primary_c, secondary_c, scale = build_composite(
        primary=primary, secondary=secondary,
        sacrifice=args.primary_sacrifice)
    ev = Evaluator(composite, dead_of(lay))
    pev = Evaluator(primary_c, dead_of(lay))
    sev = Evaluator(secondary_c, dead_of(lay))
    perm = slots_of(ev, lay)
    raw, bal, comp, usage = score_parts(ev, perm)
    pri_raw, pri_bal, pri, _ = score_parts(pev, perm)
    sec_raw, sec_bal, sec, _ = score_parts(sev, perm)
    BIG, TRI, BIG_F, TRI_F = get_tables()

    lines = []
    def out(s=""):
        print(s)
        lines.append(s)

    out(f"layout: {lay}")
    out("grid (QWERTY order, left/right split 5+5):")
    for r in range(3):
        row = lay[r * 10:(r + 1) * 10]
        out(f"  {row[:5]} | {row[5:]}")
    out(f"composite={comp:.9f} (raw={raw:.9f} balance={bal:.9f})")
    out(f"primary={pri:.9f} (raw={pri_raw:.9f} balance={pri_bal:.9f})")
    out(f"spanish={sec:.9f} (raw={sec_raw:.9f} balance={sec_bal:.9f})")
    out(f"primary_sacrifice={args.primary_sacrifice:g} secondary_scale={scale:.6g}")
    out()

    out("=== questionnaire hard probes (physical keys; * = non-default finger) ===")
    probes = [
        ("O-L", ("O", "L")),
        ("L-O", ("L", "O")),
        ("K-J-M hey", ("K", "J", "M")),
        ("F-J-M ney", ("F", "J", "M")),
        ("M-J yea", ("M", "J")),
        ("M-J-I yea", ("M", "J", "I")),
        ("H-T-J exp", ("H", "T", "J")),
        ("H-W-J ext", ("H", "W", "J")),
        ("I-U-J age", ("I", "U", "J")),
        ("N-L-J equ", ("N", "L", "J")),
        ("V-G-J ble", ("V", "G", "J")),
        ("C-O-R vid", ("C", "O", "R")),
        ("R-O-C div", ("R", "O", "C")),
        ("K-I", ("K", "I")),
    ]
    top_rows = {}
    top_by_seq = {}
    for name, seq in probes:
        text, rows = fmt_probe(seq)
        out(text)
        top_rows[name] = rows[0]
        top_by_seq[seq] = rows[0]

    checks = []
    def add_check(name, prop, detail=""):
        checks.append((name, bool(prop), detail))
        out(f"  [{'PASS' if prop else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")

    out()
    out("=== hard checks ===")
    rep_bad = []
    for ch in LETTERS:
        rows = probe_rows((ch.upper(), ch.upper()))
        if rows and rows[0][1][0] != rows[0][1][1]:
            rep_bad.append(ch)
    add_check("repeated keys use one finger", not rep_bad,
              "bad: " + ",".join(rep_bad) if rep_bad else "26/26")
    for label, seq, want in [
        ("O->L has L index", ("O", "L"), ("M", "I")),
        ("K-J-M = ring/middle/index", ("K", "J", "M"), ("R", "M", "I")),
        ("F-J-M = index/middle/index", ("F", "J", "M"), ("I", "M", "I")),
    ]:
        got = top_by_seq[seq]
        add_check(label, got[1] == want,
                  f"got {''.join(got[1])} want {''.join(want)}")
    lo = probe_rows(("L", "O"))
    add_check("L->O keeps L index within 0.2",
              any(fs[0] == "I" and cost < lo[0][0] + 0.2 for cost, fs in lo),
              f"top {''.join(lo[0][1])} / best L-index "
              f"{min((c for c, fs in lo if fs[0] == 'I'), default=0):.3f}")
    mji = top_by_seq[("M", "J", "I")][1]
    add_check("M-J-I first two strokes index", mji[0] == "I" and mji[1] == "I",
              f"got {''.join(mji)}")
    for label, seq in [("H-T-J exp", ("H", "T", "J")),
                       ("I-U-J age", ("I", "U", "J"))]:
        got = top_by_seq[seq]
        add_check(f"{label} blocks J middle", got[1][2] != "M",
                  f"got {''.join(got[1])}")
    ble = top_by_seq[("V", "G", "J")][1]
    add_check("V-G-J forces V/G index", ble[0] == "I" and ble[1] == "I",
              f"got {''.join(ble)}")
    for label, seq in [("C-O-R vid", ("C", "O", "R")),
                       ("R-O-C div", ("R", "O", "C"))]:
        got = top_by_seq[seq][1]
        add_check(f"{label} endpoints index", got[0] == "I" and got[2] == "I",
                  f"got {''.join(got)}")
    # L index gate over all neighbors except O
    lgate = []
    for n in KEY_ORDER:
        if n == "L" or n in ("O",):
            continue
        got = probe_rows(("L", n))[0][1]
        if got[0] == "I":
            lgate.append(n)
    add_check("L index only with O key (bigram)", not lgate,
              "bad: " + ",".join(lgate) if lgate else "all non-O bigram neighbours")
    out()

    out("=== corpus-wide scan (Shai, weighted) ===")
    lk = keymap(lay)
    bg_types = bg_tokens = 0
    sfb_types = sfb_tokens = 0
    imp_types = imp_tokens = 0
    alt_bg_types = alt_bg_tokens = 0
    worst_sfb = []
    worst_imp = []
    alt_usage = Counter()
    alt_examples = Counter()
    for bg, cnt in primary.B.items():
        if len(bg) != 2 or not all(c in lk for c in bg):
            continue
        bg_types += 1
        bg_tokens += cnt
        k1, k2 = lk[bg[0]], lk[bg[1]]
        i1, i2 = KEY_ORDER.index(k1), KEY_ORDER.index(k2)
        p = int(BIG_F[i1, i2])
        f1, f2 = FID2[p // 5], FID2[p % 5]
        d1, d2 = KEYS[k1].alts[0][0], KEYS[k2].alts[0][0]
        if (f1, f2) != (d1, d2):
            alt_bg_types += 1
            alt_bg_tokens += cnt
        if k1 == k2:
            if f1 != f2:
                add_check(f"repeat {bg} uses one finger", False, f"{f1}{f2}")
            continue
        if KEYS[k1].hand == KEYS[k2].hand and f1 == f2:
            sfb_types += 1
            sfb_tokens += cnt
            worst_sfb.append((cnt, bg, k1, k2, f1, BIG[i1, i2]))
        if KEYS[k1].hand == KEYS[k2].hand and impossible(k1, f1, k2, f2):
            imp_types += 1
            imp_tokens += cnt
            worst_imp.append((cnt, bg, k1, k2, f1, f2, BIG[i1, i2]))
    out(f"bigrams: {bg_types} types / {bg_tokens:.0f} tokens")
    out(f"  alt fingering: {alt_bg_types} types / {alt_bg_tokens:.0f} tokens "
        f"({pct(alt_bg_tokens, bg_tokens):.2f}%)")
    out(f"  SFB: {sfb_types} types / {sfb_tokens:.0f} tokens "
        f"({pct(sfb_tokens, bg_tokens):.2f}%)")
    out(f"  impossible: {imp_types} types / {imp_tokens:.0f} tokens "
        f"({pct(imp_tokens, bg_tokens):.2f}%)")
    if worst_sfb:
        out("  top SFBs:")
        for cnt, bg, k1, k2, f, cost in sorted(worst_sfb, reverse=True)[:10]:
            out(f"    {bg} {cnt:8.0f}  {k1}->{k2} {f}{f}  cost={cost:.2f}")
    if worst_imp:
        out("  top impossible:")
        for cnt, bg, k1, k2, f1, f2, cost in sorted(worst_imp, reverse=True)[:10]:
            out(f"    {bg} {cnt:8.0f}  {k1}->{k2} {f1}{f2}  cost={cost:.2f}")

    tri_types = tri_tokens = 0
    sfs_types = sfs_tokens = sfs_far_tokens = 0
    half_types = half_tokens = 0
    worst_sfs = []
    worst_half = []
    gate_counts = Counter()
    gate_examples = defaultdict(list)
    for tg, cnt in primary.T.items():
        if len(tg) != 3 or not all(c in lk for c in tg):
            continue
        tri_types += 1
        tri_tokens += cnt
        ks = [lk[c] for c in tg]
        ids = [KEY_ORDER.index(k) for k in ks]
        fs = [FID2[int(x)] for x in TRI_F[ids[0], ids[1], ids[2]]]
        if len(set(ks)) < 3:
            for i in range(3):
                for j in range(i + 1, 3):
                    if ks[i] == ks[j] and fs[i] != fs[j]:
                        add_check(f"trigram repeat {tg} uses one finger", False,
                                  f"{''.join(fs)}")
        if (ks[0] != ks[2] and fs[0] == fs[2]
                and KEYS[ks[0]].hand == KEYS[ks[2]].hand):
            sfs_types += 1
            sfs_tokens += cnt
            d = ansi.dist(ks[0], ks[2])
            if d > 1.3:
                sfs_far_tokens += cnt
            worst_sfs.append((cnt, tg, "".join(ks), fs[0], d))
        for i in (0, 1):
            if (ks[i] != ks[i + 1] and fs[i] == fs[i + 1]
                    and KEYS[ks[i]].hand == KEYS[ks[i + 1]].hand):
                half_types += 1
                half_tokens += cnt
                worst_half.append((cnt, tg, "".join(ks), f"{fs[i]}{fs[i+1]}"))
        for i, (k, f) in enumerate(zip(ks, fs)):
            d = KEYS[k].alts[0][0]
            neighbours = tuple(ks[j] for j in range(3) if j != i)
            alt_usage[(k, f)] += cnt
            if f == d:
                continue
            alt_examples[(k, d, f, neighbours)] += cnt
            blocked = None
            name = (k, f)
            if name == ("L", "I") and "O" not in neighbours:
                blocked = "L index without O"
            elif name == ("G", "M"):
                blocked = "G middle"
            elif name == ("V", "M") and "G" in neighbours:
                blocked = "V middle with G"
            elif name == ("C", "M") and "R" in neighbours:
                blocked = "C middle with R"
            elif name == ("J", "M") and any(x in ("T", "I", "G") for x in neighbours):
                blocked = "J middle with T/I/G"
            elif name == ("M", "M") and "J" in neighbours:
                blocked = "M middle with J"
            if blocked:
                gate_counts[blocked] += cnt
                if len(gate_examples[blocked]) < 5:
                    gate_examples[blocked].append(
                        (tg, cnt, "".join(fs), "".join(ks)))
    out(f"trigrams: {tri_types} types / {tri_tokens:.0f} tokens")
    out(f"  SFS: {sfs_types} types / {sfs_tokens:.0f} tokens "
        f"({pct(sfs_tokens, tri_tokens):.2f}%; far {sfs_far_tokens:.0f})")
    out(f"  half-SFB: {half_types} types / {half_tokens:.0f} tokens "
        f"({pct(half_tokens, tri_tokens):.2f}%)")
    if worst_sfs:
        out("  top SFS:")
        for cnt, tg, ks, f, d in sorted(worst_sfs, reverse=True)[:10]:
            out(f"    {tg} n={cnt:8.0f}  {ks} {f}{f}  dist={d:.2f}")
    if worst_half:
        out("  top half-SFB:")
        for cnt, tg, ks, fs in sorted(worst_half, reverse=True)[:10]:
            out(f"    {tg} n={cnt:8.0f}  {ks} {fs}")
    if gate_counts:
        out("  alt-gate violations:")
        for k, v in gate_counts.most_common():
            out(f"    {k}: {v:.0f} tokens ({pct(v, tri_tokens):.3f}%)")
            for tg, cnt, fs, ks in gate_examples[k]:
                out(f"      {tg} n={cnt:.0f} keys={ks} fingers={fs}")
    else:
        out("  alt-gate violations: none")
    alt_tokens = sum(v for (k, f), v in alt_usage.items()
                     if f != KEYS[k].alts[0][0])
    out(f"  actual alt strokes: {alt_tokens:.0f} / "
        f"{3 * tri_tokens:.0f} ({pct(alt_tokens, 3 * tri_tokens):.2f}%)")

    out("  top actual alt contexts (trigram table):")
    for (k, d, f, neigh), cnt in alt_examples.most_common(20):
        out(f"    {k}:{d}->{f} around {''.join(neigh)}  n={cnt:.0f}")
    out("  alt use by key (share of trigram strokes):")
    total_key = Counter()
    for (k, f), v in alt_usage.items():
        total_key[k] += v
    rows = []
    for k in KEY_ORDER:
        if k not in total_key:
            continue
        alt = sum(v for (kk, f), v in alt_usage.items()
                  if kk == k and f != KEYS[k].alts[0][0])
        rows.append((alt / total_key[k], k, alt, total_key[k]))
    for share, k, alt, tot in sorted(rows, reverse=True)[:12]:
        out(f"    {k}: {share*100:5.1f}%  ({alt:.0f}/{tot:.0f})")
    out()

    out("=== composite score decomposition ===")
    out(f"  raw trigram cost = {comp - bal:.9f}; balance penalty = {bal:.9f}")
    out("  finger load (default fingers, Shai letter freq):")
    labels = ["P", "R", "M", "I"]
    for hand in "LR":
        shares = [usage[hand][i] / sum(usage[hand]) * 100 for i in range(4)]
        out(f"    {hand}: " + "  ".join(f"{labels[i]}={shares[i]:5.1f}%"
                                       for i in range(4)))
    out()

    out("=== corpus sensitivity (calibrated scorer2, total score) ===")
    corpus_scores = {}
    for cname, cpath in CORPORA.items():
        corpus = Corpus.load(cpath)
        row = {}
        for name, clay in REFS.items():
            try:
                row[name] = score_layout(corpus, clay)["total"]
            except Exception as ex:
                row[name] = None
        corpus_scores[cname] = row
        order = sorted((v, k) for k, v in row.items() if v is not None)
        best_name, best_score = order[0][1], order[0][0]
        rank = [k for _, k in order].index("shai_best") + 1
        delta = row["shai_best"] - best_score
        out(f"  {cname:12s} best={best_name:14s} {best_score:.6f}  "
            f"shai_best={row['shai_best']:.6f}  rank={rank}/{len(order)}  "
            f"delta={delta:+.6f}")
    out()

    out("=== reference scores on Shai primary ===")
    for name, clay in REFS.items():
        evx = Evaluator(primary_c, dead_of(clay))
        px = score_parts(evx, slots_of(evx, clay))[2]
        evs = Evaluator(secondary_c, dead_of(clay))
        sx = score_parts(evs, slots_of(evs, clay))[2]
        evc = Evaluator(composite, dead_of(clay))
        cx = score_parts(evc, slots_of(evc, clay))[2]
        out(f"  {name:14s} comp={cx:.9f} primary={px:.9f} spanish={sx:.9f}  {clay}")
    out()
    out(f"wrote {args.out} and {args.out_json}  ({__import__('time').time()-t0:.1f}s)")

    payload = {
        "layout": lay,
        "grid": [lay[r*10:(r+1)*10] for r in range(3)],
        "scores": {"composite": comp, "raw": raw, "balance": bal,
                   "primary": pri, "primary_raw": pri_raw,
                   "primary_balance": pri_bal, "spanish": sec,
                   "spanish_raw": sec_raw, "spanish_balance": sec_bal},
        "probes": {name: {"seq": seq, "cost": top_rows[name][0],
                          "fingers": top_rows[name][1]}
                   for name, seq in probes},
        "checks": [{"name": n, "pass": ok, "detail": d} for n, ok, d in checks],
        "scan": {"bigram_types": bg_types, "bigram_tokens": bg_tokens,
                 "alt_bigram_tokens": alt_bg_tokens,
                 "sfb_types": sfb_types, "sfb_tokens": sfb_tokens,
                 "impossible_types": imp_types,
                 "impossible_tokens": imp_tokens,
                 "trigram_types": tri_types, "trigram_tokens": tri_tokens,
                 "sfs_types": sfs_types, "sfs_tokens": sfs_tokens,
                 "sfs_far_tokens": sfs_far_tokens,
                 "half_sfb_types": half_types,
                 "half_sfb_tokens": half_tokens,
                 "alt_gate_violations": dict(gate_counts)},
        "finger_load": {hand: [usage[hand][i] / sum(usage[hand])
                               for i in range(4)] for hand in "LR"},
        "corpus_scores": corpus_scores,
    }
    Path = __import__("pathlib").Path
    Path(args.out).write_text("\n".join(lines) + "\n")
    Path(args.out_json).write_text(json.dumps(payload, indent=1) + "\n")


if __name__ == "__main__":
    main()
