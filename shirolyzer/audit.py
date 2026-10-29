#!/usr/bin/env python3
"""Questionnaire and ruling audit for the pattern model.

usage: python3 shirolyzer/audit.py [--config A:all] [--out PATH]"""
import argparse, sys, os, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np
import comfort_geo as cg, gates as GT, fitlib as FL, rulings as RL
import scorer_geo as SG
from comfort_geo import KI, KEYS, FINGERS, NF

ap = argparse.ArgumentParser(description=__doc__)
ap.add_argument("config", nargs="?", default="A:all")
ap.add_argument("--out", default=None)
args = ap.parse_args()
cfgname = args.config
model, gname = SG.load_model(cfgname)
tabs = model.tables()
DEFAULT = cg.DEFAULT_FINGER


def probe_rows(seq):
    ids = [KI[k] for k in seq]
    t = (cg.bi_many(tabs, [ids[0]], [ids[1]])[0] if len(ids) == 2 else cg.tri_block(model, tabs, *ids)) + SG.PRESS * len(ids)
    t = t + GT.penalty_mask(tuple(seq))
    rows = [(float(t[x]), "".join(FINGERS[f] for f in x)) for x in np.ndindex(t.shape) if np.isfinite(t[x])]
    rows.sort()
    return rows


def fmt(seq, limit=5):
    rows = probe_rows(seq)
    out = [f"  {''.join(seq)}:"]
    for c, fs in rows[:limit]:
        out.append(f"    {c:8.3f}  " + " -> ".join(f"{k}:{f}{'*' if f != DEFAULT[k] else ''}" for k, f in zip(seq, fs)))
    return "\n".join(out), rows


lines = []
def out(s=""):
    print(s); lines.append(s)

out(f"config {cfgname}  (explicit alt-licence table: {sorted(GT.gate_subset('explicit'))})")
out("=== questionnaire hard probes (physical keys; * = non-default finger) ===")
probes = [("O-L", "OL"), ("L-O", "LO"), ("K-J-M hey", "KJM"), ("F-J-M ney", "FJM"), ("M-J yea", "MJ"), ("M-J-I yea", "MJI"),
          ("H-T-J exp", "HTJ"), ("H-W-J ext", "HWJ"), ("I-U-J age", "IUJ"), ("N-L-J equ", "NLJ"), ("V-G-J ble", "VGJ"),
          ("C-O-R vid", "COR"), ("R-O-C div", "ROC"), ("K-I", "KI"),
          ("th W-D", "WD"), ("ht D-W", "DW"), ("as A-S", "AS"), ("sa S-A", "SA"), ("X-A", "XA"), ("A-X", "AX"),
          ("U-J", "UJ"), ("U-H", "UH"), ("X-W", "XW"), ("S-W", "SW"), ("W-X-F wxf", "WXF"), ("F-X-W fxw", "FXW"),
          ("Q-S", "QS"), ("S-Q", "SQ"), ("O-K", "OK"), ("K-O", "KO"), ("K-;", "K;"), (";-K", ";K"), ("A-E", "AE"), ("E-A", "EA"), ("T-G", "TG"), ("G-T", "GT"), ("Y-U", "YU"), ("U-Y", "UY"), ("D-A", "DA"), ("A-D", "AD"), ("F-S", "FS"), ("S-F", "SF"), ("O-J", "OJ"), ("J-O", "JO")]
top = {}
for name, seq in probes:
    txt, rows = fmt(tuple(seq)); out(txt); top[seq] = rows[0][1]
checks = []
def chk(name, ok, detail=""):
    checks.append(ok); out(f"  [{'PASS' if ok else 'FAIL'}] {name}  {detail}")
out(); out("=== hard checks ===")
rep_bad = [ch for ch in "QWERTYUIOPASDFGHJKLZXCVBNM" if probe_rows((ch, ch))[0][1][0] != probe_rows((ch, ch))[0][1][1]]
chk("repeated keys use one finger", not rep_bad, f"bad: {rep_bad}" if rep_bad else "26/26")
chk("O->L has L index", top["OL"] == "MI", top["OL"])
chk("K-J-M = ring/middle/index", top["KJM"] == "RMI", top["KJM"])
chk("F-J-M = index/middle/index", top["FJM"] == "IMI", top["FJM"])
lo = probe_rows(("L", "O")); best_i = min(c for c, f in lo if f[0] == "I")
chk("L->O keeps L index within 0.2", best_i < lo[0][0] + 0.2, f"top {lo[0][1]} / best L-index {best_i:.3f} vs {lo[0][0]:.3f}")
chk("M-J-I first two strokes index", top["MJI"][:2] == "II", top["MJI"])
chk("H-T-J blocks J middle", top["HTJ"][2] != "M", top["HTJ"])
chk("I-U-J blocks J middle", top["IUJ"][2] != "M", top["IUJ"])
chk("V-G-J forces V/G index", top["VGJ"][:2] == "II", top["VGJ"])
chk("C-O-R endpoints index", top["COR"][0] == "I" and top["COR"][2] == "I", top["COR"])
chk("R-O-C endpoints index", top["ROC"][0] == "I" and top["ROC"][2] == "I", top["ROC"])
lg = [n for n in cg.KEYS if n not in ("L", "O") and probe_rows(("L", n))[0][1][0] == "I"]
chk("L index only with O key (bigram)", not lg, f"bad: {lg}" if lg else "all non-O neighbours")
out(); out("=== 2026-10-27 answers ===")
chk("W->D ring/middle (th)", top["WD"] == "RM", top["WD"])
chk("D->W middle/ring is cheaper than W->D", probe_rows(("D", "W"))[0][0] < probe_rows(("W", "D"))[0][0] and top["DW"] == "MR", top["DW"])
chk("A->S pinky/ring (2026-10-29 correction)", top["AS"] == "PR", top["AS"])
chk("S->A ring/pinky", top["SA"] == "RP", top["SA"])
_c = {f: c for c, f in probe_rows(("X", "A"))}
chk("X->A index/ring first and cheaper than middle/ring; ring X + pinky A impossible", top["XA"] == "IR" and top["AX"] == "RI"
    and _c["IR"] < _c["MR"] and min(c for c, f in probe_rows(("X", "A")) if f == "RP") >= 8.0, top["XA"] + " " + top["AX"])
chk("Q-S = middle/index, S-Q = index/middle", top["QS"] == "MI" and top["SQ"] == "IM", top["QS"] + " " + top["SQ"])
chk("O->K ring/middle, K->O middle/ring", top["OK"] == "RM" and top["KO"] == "MR", top["OK"] + " " + top["KO"])
chk("K->; middle/pinky, ;->K pinky/middle", top["K;"] == "MP" and top[";K"] == "PM", top["K;"] + " " + top[";K"])
chk("A->E pinky/middle, E->A middle/pinky", top["AE"] == "PM" and top["EA"] == "MP", top["AE"] + " " + top["EA"])
_rest = [("A", "P"), ("W", "R"), ("E", "M"), ("F", "I")], [(";", "P"), ("O", "R"), ("I", "M"), ("J", "I")]
_bad = [a + b for L in _rest for a, fa in L for b, fb in L if a != b and probe_rows((a, b))[0][1] != fa + fb]
chk("defaults first: both keys at rest -> rest fingers (24 pairs)", not _bad, f"bad: {_bad}" if _bad else "24/24")
chk("U->J, U->H middle on U", top["UJ"] == "MI" and top["UH"] == "MI", top["UJ"] + " " + top["UH"])
chk("X->W and S->W index/middle", top["XW"] == "IM" and top["SW"] == "IM", top["XW"] + " " + top["SW"])
chk("wxf = MII", top["WXF"] == "MII", top["WXF"])
chk("fxw does not use the M/R full-span scissor", top["FXW"] != "IMR", top["FXW"])
chk("T->G middle/index, G->T index/middle", top["TG"] == "MI" and top["GT"] == "IM", top["TG"] + " " + top["GT"])
chk("YU and RT/TR ring-middle / middle-ring on both hands (Y->U MR, U->Y RM, R->T RM, T->R MR)", top["YU"] == "MR" and top["UY"] == "RM" and probe_rows(("R", "T"))[0][1] == "RM" and probe_rows(("T", "R"))[0][1] == "MR", top["YU"] + " " + top["UY"] + " " + probe_rows(("R", "T"))[0][1] + " " + probe_rows(("T", "R"))[0][1])
chk("D->A middle/pinky, A->D pinky/middle", top["DA"] == "MP" and top["AD"] == "PM", top["DA"] + " " + top["AD"])
chk("F->S index/ring, S->F ring/index; O->J ring/index, J->O index/ring", top["FS"] == "IR" and top["SF"] == "RI" and top["OJ"] == "RI" and top["JO"] == "IR", " ".join(top[k] for k in ("FS", "SF", "OJ", "JO")))
_c = {f: c for c, f in probe_rows(("X", "A"))}
chk("X->A order: index/ring < middle/ring < index/middle < any pinky fingering; index/pinky never", _c.get("IP", 99) >= 8 and _c["IR"] < _c["MR"] < _c["IM"] < _c["MP"], f"IR {_c['IR']:.2f} MR {_c['MR']:.2f} IM {_c['IM']:.2f} MP {_c['MP']:.2f} IP {_c.get('IP', 99):.2f}")
_ipbad = [(a + b) for a in cg.KEYS for b in cg.KEYS if a != b and (a in "QWERTASDFGZXCVB") == (b in "QWERTASDFGZXCVB")
          and cg.PF_S(a, "P", b, "I") < 1.9 and model.tables()["PT"][cg.KI[a], cg.FI["P"], cg.KI[b], cg.FI["I"]] < 8.0]
chk("index + pinky needs >= 1.9 key widths between the keys (pattern)", not _ipbad, f"bad: {_ipbad[:5]}" if _ipbad else "all such pairs impossible")
def _best(a, b): return probe_rows((a, b))[0][1]
chk("2026-10-29: ;->U PM, U->; MP; T->F IM, F->T MI; ;->L PR, L->; RP", _best(";", "U") == "PM" and _best("U", ";") == "MP" and _best("T", "F") == "IM" and _best("F", "T") == "MI" and _best(";", "L") == "PR" and _best("L", ";") == "RP", " ".join(_best(a, b) for a, b in ((";", "U"), ("U", ";"), ("T", "F"), ("F", "T"), (";", "L"), ("L", ";"))))
_c2 = {f: c for c, f in probe_rows((";", "U"))}; _c3 = {f: c for c, f in probe_rows(("U", ";"))}
chk(";-U with ring + middle is borderline impossible (>= 5)", _c2["RM"] >= 5.0 and _c3["MR"] >= 5.0, f"{_c2['RM']:.2f} {_c3['MR']:.2f}")
chk("A->S PR and the ring/pinky pair is not scissor-penalised away (S->A RP)", _best("A", "S") == "PR" and _best("S", "A") == "RP", _best("A", "S") + " " + _best("S", "A"))
import numpy as _np
_E = model.tables()["E"]
_bad = [k for k in cg.KEYS if cg.FINGERS[int(_np.argmin(_E[cg.KI[k]]))] != cg.DEFAULT_FINGER[k]]
chk("isolated key -> your default fingermap (all 30 keys + ;)", not _bad, f"bad: {_bad}" if _bad else "all keys")
_T = model.tables()["T2"]
_asym = []
for _a in cg.KEYS:
    for _b in cg.KEYS:
        if _a == _b: continue
        for _fa in range(4):
            for _fb in range(4):
                if _fa == _fb: continue
                _x, _y = _T[cg.KI[_a], _fa, cg.KI[_b], _fb], _T[cg.KI[_b], _fb, cg.KI[_a], _fa]
                if _np.isfinite(_x) and _np.isfinite(_y) and abs(_x - _y) > 0.02 and "".join(sorted(cg.FINGERS[_fa] + cg.FINGERS[_fb])) not in ("MR", "PR"):
                    _asym.append((_a + _b, cg.FINGERS[_fa] + cg.FINGERS[_fb]))
chk("A->B and B->A price alike except ring/middle and pinky/ring pairs (direction rules)", not _asym, f"{len(_asym)} asymmetric, e.g. {_asym[:3]}" if _asym else "symmetric")
out(f"probes+checks: {sum(checks)}/{len(checks)}")

out(); out("=== every ruling (legacy + round 8) ===")
cfg = FL.Cfg(True, gname)
GT.set_explicit(gname)
_, det = FL.total_loss(model.theta, cfg, return_detail=True)
anch = [d for d in det if d[0].id == "anchor_default_pairs"]
det = [d for d in det if d[0].id != "anchor_default_pairs"]
for r, ok, l, txt in det:
    out(f"  [{'PASS' if ok else 'FAIL'}] {r.id:16s} {txt}")
out(f"rulings: {sum(d[1] for d in det)}/{len(det)}")
for d in anch: out(f"anchor (not a ruling): {d[3]}")
out_path = args.out or os.path.join(os.path.dirname(os.path.abspath(__file__)), "results", f"audit_{cfgname.replace(':','_')}.txt")
os.makedirs(os.path.dirname(os.path.abspath(out_path)), exist_ok=True)
open(out_path, "w").write("\n".join(lines))
