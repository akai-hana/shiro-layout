# Human-observation tuning — what's left after the machine floor

## 1. Why this exists

Machine search is at a hard floor **of our objective** (confirmed: no 1/2/3-swap
or ILS improves `4.3877`). But every top community layout got **post-release
human swaps** the objective couldn't justify:

- **Gallium v2**: `X↔Q` (QX never occurs) + `J↔Z` (lowers SFBs/SFS). Small,
  rare-letter, geometry-motivated. Kept because they win under *standard
  fingering* (the keysolve view) at ~0 objective cost.
- **Sturdy**: hand-tuned for **bad-redirect feel** (Oxey: "bad redirects = any
  redirect not involving index"); known annoyance `sk` (against finger
  length), `rl` roll overrepresentation. Feel-driven, not score-driven.
- **Graphite**: tried hand-swaps, **reverted nearly all** ("I was not smarter
  than Oxeylyzer"); kept only what the analyzer *couldn't see* (punctuation +
  shift-layer bigrams).

The meta-rule: a human swap is justified only when the objective is **flat**
(Δ ≈ 0) *and* a felt/geometry reason exists. If Δ > ~0.005, the machine already
rejected it and the human should trust the machine (Graphite's lesson).

## 2. cfgE anomaly audit (measured — `results/human_audit.txt`)

Per-key load + finger-group (letters+punct):
```
LI index 17.2 · LM middle 16.9 · RI index 15.9 · RM middle 15.1
LR ring 10.6 · RR ring 10.6 · LP pinky 7.2 · RP pinky 6.6
```
Hand balance near-parity (left 51 / right 49). Index/middle-heavy by design (B5).

**Rest keys (from `fingers.py`, the arched `AWEF/JIO;` model):**
```
W=LR  E=LM  A=LP  F=LI  |  J=RI  I=RM  O=RR  ;=RP
```
Ring/middle rest on the **top row** (W/E, O/I); index/pinky rest on home
(F/A, J/;). This is the whole point of the arched posture — top-row ring/middle
are *rest*, not reaches, and the pinkies never extend upward.

**Anomaly ranking (load × geometry-effort), true anomalies only:**
The first audit used a standard-ASDF heuristic (top-row + ring/pinky ⇒ stretch)
and **falsely flagged `a` on W, `r` on O, `i` on E, `n` on I, `o` on R, `l` on
U** — all rest keys or the ring/middle fingers' own laterally-adjacent columns
(`R`/`U` are the middle/ring "own-column" keys per README rule 2/3). Corrected
to exclude rest keys and home row:

| letter | load | key | finger | row | note |
|---|---|---|---|---|---|
| `o` | 7.6% | `R` | LM | top | middle's own adjacent column — not a stretch |
| `d` | 4.3% | `M` | RI | bot | index bottom, fine |
| `l` | 3.9% | `U` | RM | top | ring's own stretch column |
| `u` | 2.7% | `V` | LI | bot | index bottom |
| `w` | 2.0% | `P` | RR | top | **ring top, 1 alt — real geometry flag** |
| `b` | 1.4% | `Z` | LP | bot | **pinky bottom — Kuntem flag** |

- `s` on `;` (6.6%) and `h` on `A` (5.8%) sit on pinky **rest** keys — the
  design's stated philosophy (pinky homes a high-frequency letter, low
  movement). Not a bug; a human *could* disagree (Canary puts nothing >4% on
  pinky) but that is a philosophy change, not a feel swap.
- **`w` on `P`** is the one genuine geometry flag: right-ring top column with a
  single alt and no second rescue. It is the Sturdy/Canary-class annoyance
  (`WR` scissor, `was`/`way`/`with`). Moving it costs +0.024 — machine rejected.
- **`b` on `Z`** is the Kuntem flag (pinky bottom) but only 1.4% load; `bz`
  costs +0.079 — machine rejected.

There is **no common letter on a genuinely bad key** once the rest model is
respected. The layout's letter placement is clean under its own posture.

## 3. Machine-free single swaps (ΔObj ≤ +0.003), full census

| pair | ΔObj | ΔStdSFB | ΔPinky | ΔLat | geometry Δ | note |
|---|---|---|---|---|---|---|
| `qx` | +0.0007 | +0.061 | 0 | 0 | **−0.06** | Q off hard bot-inner-index → free (best geometry) |
| `jq` | +0.0011 | **−0.089** | 0 | 0 | −0.003 | **wins under keysolve view too** (Gallium J↔Z class) |
| `jx` | +0.0018 | +0.071 | 0 | 0 | −0.033 | J off top-inner stretch |
| `jz` | +0.0013 | — | 0 | 0 | −0.005 | rare↔rare |
| `qz` | +0.0020 | 0.000 | 0 | 0 | 0 | Gallium J↔Z logic (rare↔rare) |

All five involve only `q`/`j`/`x`/`z` — the four rarest letters (each <0.3%).
**No common letter has a machine-free move.** `w`-off-pinky moves cost
+0.024…+0.106 (real, machine-rejected).

## 4. Alt-fingering discount check (the last theoretical blind spot)

If the model over-discounts a same-finger pair via alts, a human might hand-swap
it. Measured (default-table cost − alt-table cost = discount):

| pair | freq% | stdCost | ourCost | discount | fingers |
|---|---|---|---|---|---|
| `io` | 0.64 | 0.053 | 0.015 | **0.038** | MM (ring+middle, cheap alt) |
| `on` | 1.64 | 0.052 | 0.042 | 0.011 | MM |
| `co` | 0.73 | 0.029 | 0.024 | 0.005 | MM |
| `in` | 2.47 | 0.051 | 0.051 | **0** | MM adjacent — no rescue |
| `ed` | 1.37 | 0.035 | 0.035 | **0** | II adjacent |
| `te` | 1.09 | 0.021 | 0.021 | **0** | II adjacent |
| `ar` | 1.02 | 0.027 | 0.027 | **0** | RR adjacent |
| `sh` | 0.44 | 0.015 | 0.015 | **0** | PP adjacent |

**No over-discount anomaly**: the top-SFBs (`in`,`ed`,`te`,`ar`,`sh`) get zero
rescue because they're physically adjacent/same-row same-finger — the model
charges full price. The discounted ones (`io`,`on`) are cheap *because* they're
genuinely alt-able (top-row middle+ring). So the alt model is honest, not a
blind spot.

## 5. Verdict — what a human *could* do, and the recommendation

**There is no "free feel swap" for cfgE.** Unlike Gallium (which had rare-letter
geometry wins like `X↔Q` at Δ≈0 that also beat the standard-finger view), cfgE's
machine-free region contains only `q`/`j`/`x`/`z` shuffles, and the only one that
*also* wins the standard-finger view is **`jq`** (ΔObj +0.0011, ΔStdSFB −0.089):

```
jq variant:  ;aioqvlnrwhfpeygtcmsbxjuzkd,./     (cfgE with J↔Q)
```
- What it buys: `j` leaves top-inner-index (a stretch) onto `q`'s bot-inner-index;
  `q` moves to top-inner-index. Wins under keysolve standard view; free under ours.
- What it costs: +0.0011 objective (noise). `q` (0.15%) and `j` (0.15%) are so
  rare that **no typist will ever feel the difference.** It is a *theater* swap —
  strictly non-worse, but not felt.

**Recommendation: ship cfgE unchanged.** The human-observation avenue, which
genuinely helped Gallium/Sturdy, yields **nothing felt** for cfgE:
- Under the **arched rest model** (`AWEF/JIO;`), the letters that look "high"
  (`a` on W, `i` on E, `o` on R, `r` on O, `n` on I, `l` on U) are all on rest
  keys or the ring/middle fingers' own adjacent columns — comfortable by
  design, not anomalies. The first audit's "pinky-rescue" flag for `a`-on-W was
  a bug: the pinky never reaches W (it's the ring's rest), so there is nothing
  to rescue.
- The only true geometry flags left are `w` on `P` (ring top, +0.024 to fix)
  and `b` on `Z` (pinky bottom, +0.079 to fix) — both already priced and
  rejected by the machine.
- `s` on `;` and `h` on `A` sit on pinky rest keys — the design's stated
  philosophy, not defects.
- The machine-free swaps are all rare-letter (`q`/`j`/`x`/`z`) theater.

This is the *good* outcome of the Graphite lesson: the machine found what a human
would have found, and the human has nothing to add. cfgE is done.

## 6. If the user still wants a human-tuned variant

The only defensible move is **`jq`** (Δ +0.0011, wins standard-finger view,
geometry-neutral-to-positive). It would be labeled `cfgE-hum` and shipped as an
*equal*, not an improvement. Not recommended (no felt benefit), but available:

```
cfgE-hum = ;aioqvlnrwhfpeygtcmsbxjuzkd,./
```
(`/home/akai/git/layout-opencode-investigation/analysis/human_shortlist.py`,
`human_geometry.py` regenerate the tables.)
