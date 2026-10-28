# Keyboard Layout Research Notes

Compiled 2026-10-19 from live fetches (curl + Mozilla/5.0). Primary sources:
- layouts.wiki (AKL community wiki; URL pattern `/layouts/<year>/<name>/`)
- Creator repos: binarybottle/engram(-2021), Apsu/Canary, GalileoBlues/Gallium, O-X-E-Y/*
- SteveP: stevep99.github.io, colemakmods.github.io (old stevep99.info is NXDOMAIN/dead)
- Pascal Getreuer's alt-layouts guide (getreuer.info)
- GalileoBlues/Layout-Wiki (AKL Discord archive: Terminology + AKL Layouts)
- Norvig-style letter stats approximated from google-10000-english word list (see §4 caveats)

Raw downloads live in `/tmp/opencode/research/pages/` (`lw_*` = layouts.wiki pages, `ref_*` = wiki reference, `gh_*` = GitHub). Machine-extracted grids: `keyboard_grids.json`. Parsed stats: `all_layout_stats.json`. Extracted per-layout summaries: `extracted/`.

---

## 0. Notation

Layouts written as 3 rows × 10 columns (left 5 + right 5), QWERTY-position diagram style:

```
top:   T0 T1 T2 T3 T4    T5 T6 T7 T8 T9
home:  H0 H1 H2 H3 H4    H5 H6 H7 H8 H9
bot:   B0 B1 B2 B3 B4    B5 B6 B7 B8 B9
```

Finger map (traditional touch typing, left→right):
- Top/home/bot columns 0–4: pinky, ring, middle, index, index-stretch (inner index)
- Columns 5–9: index-stretch (inner index), index, middle, ring, pinky

**Angle Mod** (CRITICAL for Canary/Sturdy-class layouts): on row-staggered boards, reassign left bottom row so ring→QWERTY-Z, middle→X, index→C+V (letters physically shifted one key left). All reported stats for those layouts assume Angle Mod unless noted.

Home rows below are the **middle keyboard row** letters by finger (not a "rest position" claim beyond what sources state).

---

## 1. Reference layouts

### 1.1 Colemak (Shai Coleman, 2006)

```
q w f p g   j l u y ;
a r s t d   h n e i o '
z x c v b   k m , . /
```

**Home row by finger:** L: A(pinky) R(ring) S(middle) T(index) D(index-str) | R: H(index-str) N(index) E(middle) I(ring) O(pinky)

**Rationale (layouts.wiki):** Third most popular English layout; large community. Questioned Dvorak's hand-alternation preference, preferring **rolls**. Kept QWERTY ZXCV for hotkeys/compat; punctuation largely unmoved except `;`. Most recent alt layout shipped by default on major OSes.

**Stats (layouts.wiki, traditional finger map):**
| Metric | Value |
|---|---|
| SFB | 0.96% (P68) |
| Full scissor | 0.09% |
| Half scissor | 1.92% |
| Lat. stretch | 2.04% (P97 — high!) |
| SFS (skipgram same-finger) | 6.01% |
| Weak redirects | 1.52% |
| Rolls : Alts | 2.20 |
| Finger usage L→R | 6.55 / 6.10 / 8.90 / 16.15 / — / 15.88 / 12.64 / 9.07 / 7.47 % |

SteveP compare tool (colemakmods): Colemak traditional overall effort/key 1.836; with ergonomic (angle) fingering 1.802; SFB 1.67% on his corpus/model.

### 1.2 Colemak-DH / Colemak Mod-DH (SteveP, 2014)

Angle-mod matrix form (recommended):
```
q w f p b   j l u y ;
a r s t g   m n e i o '
z x c d v   k h , . /
```

**Home row:** L: A R S T G(index-str) | R: M(index-str) N E I O  
Changes vs vanilla Colemak: **G↔M** onto home index-stretch columns; **D** moved to bottom (between C and V); **H** moved to bottom-right index area — eliminates center-column lateral stretches (LSB).

**Rationale:** Get rid of D/H center-column LSBs ("d and h off the center columns"). Ergonomic Angle Mod recommended (SteveP's ergonomic-mods/angle.html).

**Stats (Angle Mod variant on layouts.wiki):** SFB 0.94%, FSB 0.09%, HSB 1.89%, **LSB 1.10%** (vs Colemak 2.04%), SFS 6.03%, Rolls:Alts 2.21, Weak redir 1.52%.

SteveP compare: Colemak-DH matrix 1.726 overall (best on that page); ergonomic 1.735; SFB 1.67%.

AKL modern consensus (Getreuer, layouts.wiki recs): solid, very low off-home pinky use, keeps QWERTY ZXC(V) hotkeys — but **superseded on metrics** by Gallium/Graphite/Sturdy/Canary generation.

### 1.3 Engram (Arno Klein / binarybottle)

**Classic Engram v2.0 (2021-12-18)** — the widely known version:
```
b y o u '   " l d w v z
c i e a ,   . h t n s q
g x j k -   ! r m f p
```
(ASCII from binarybottle/engram-2021 README; layouts.wiki 2025 page still shows this as the 2021 variant.)

**Home row (classic):** L: C(pinky) I(ring) E(middle) A(index) ,(index-str) | R: .(index-str) H(index) T(middle) N(ring) S(pinky)  
Q is bottom-right pinky; punctuation `"` `,` `.` `!` occupies the two center columns.

**Engram-en v2025.10.31** (new algorithm: crowdsourced typing preferences + multi-objective optimization; peer-reviewed IJHCI paper "Optimizing comfortable keyboard layouts…"):
```
b y o u '   " d n g v q
h i a e ,   . t r s c z
k j x w -   ? m l f p
```
**Home row (2025):** L: H I A E , | R: . T R S C

**Design criteria (from 2021 README, numbered):**
1. No lateral finger movements (one column per finger; middle columns reserved for punctuation).
2. Alternate hands over uncomfortable same-hand transitions.
3. Most common letters on most comfortable keys.
4. Frequent bigrams easier to type.
5. **Little→index (in) rolls preferred over index→little (out).**
6. Balance finger loads to relative strength.
7. Avoid stretching shorter fingers up / longer fingers down.
8. Avoid same finger.
9. Avoid skipping home row.
10. Common punctuation in middle columns.
11. Memorable Shift-numbers.

**Key geometric claim (directly relevant to "high home row"):** Engram treats top-center keys (ring/middle columns of top row, positions 2–3 and 14–15) as among the **most comfortable**, because "longer middle and ring fingers can uncurl upwards." Bottom-corner keys (9,12 / 21,24) are also comfortable (short fingers curl down). So B Y O U on top and C I E A / H I A E on home is intentional comfort ranking, **not** a rest-on-top-row design.

**Scores (2021 README, Norvig/Google bigrams ×100):** Engram 62.48 > Halmak 62.40 > … > Colemak 62.36 > QWERTY 62.19.  
KLA (patorjk) scores: Engram highest on 7/9 prose corpora (e.g. Alice 70.13 vs Colemak 65.83, QWERTY 53.06).  
Same-finger consecutive presses (Alice): Engram 216 vs Colemak 362, QWERTY much higher.  
Inward-roll frequency (billions, Google): Engram 4.64 (32-key) / 4.51 (24-key) — top of table.

**layouts.wiki stats — Engram 2025 variant:** SFB 1.38%, FSB 0.30%, HSB 1.32%, **LSB 0.18%** (very low), SFS 4.46%, Rolls:Alts 1.50, Weak redir 0.28%.  
**Engram 2021 variant:** SFB ~1.30%, FSB 0.14%, HSB 0.85%; Rolls:Alts 2.05 (more roll-heavy); 3-roll In:Out extreme 6.44 (strongly in-roll biased on 3-keys).

### 1.4 Canary (AKL community / popularized by Apsu, 2022)

Origin: collaborative layout from top AKL Discord creators; released with a MonkeyType typing tournament. layouts.wiki credits author Apsu 2022. (User's "word count optimizer result from Oxey/community": Canary emerged from community optimization discussion; oxeylyzer/genkey-era methods; the public writeup is Apsu's GitHub.)

**Ortho / base form (GitHub + layouts.wiki):**
```
w l y p b   z f o u ;
c r s t g   m n e i a
q j v d k   x h / , .
```

**Angle-mod row-stagger form (GitHub):**
```
w l y p k   z x o u ;
c r s t b   f n e i a
j v d g q   m h / , .
```

**Home row (ortho):** L: C R S T G | R: M N E I A  
**Home row (angle-mod):** L: C R S T B | R: F N E I A

**Design principles (GitHub, condensed):**
- **Primary goal: extremely high rolls** — "over 55% of all Trigrams" are rolls. Flowy feel vs alternation-heavy layouts.
- Does **not** heavily favor in- vs out-rolls (anecdotal that in-rolls are faster).
- `T` left index + `N` right index so high-freq letters are roll *targets*.
- **Near-zero pinky SFBs**, minimal pinky redirects (pinky length varies hugely across population).
- All primary vowels on one hand; vowel block minimizes SFBs *and* redirects without overloading pinky.
- Semivowels W/Y on opposite hand from vowels, avoiding `ly ty tw sw` clashes. W on top-left pinky (Angle Mod frees that finger); Y in `ysv` column.
- Colemak-DH similarity intentional; fixes: avoid `eve` redirect (K/V placement), avoid `you` as top→home→top redirect (Y not in vowel cluster), prefers `oe` over `ue` SFB column, middle>ring load.
- L and R ("liquids") on ring fingers (like Semimak) — low L·R with each other, mid frequency.
- `'` keeps QWERTY spot: combines with almost everything *except* A.
- Hardest row-stagger keys (QWERTY B and Y positions) get rarest letters Q and Z. Q bottom so `QU` is cross-hand (vowels on right).

**Stats (layouts.wiki, Angle Mod):** SFB 0.67%, FSB 0.04%, **HSB 0.72%**, LSB 0.55%, SFS 5.62% (higher — concentrated on overloaded fingers), Rolls:Alts 2.03, **Weak redirects 0.09%** (excellent). Finger usage: L pinky only 3.74% (very low); L index 16.18%, R index 15.96%.

**Known issues (AKL recs):** scissor **WR** ("write", "wrong"); uneven finger usage → high SFS on L index / R index / R middle.

### 1.5 Sturdy (Oxey, 2021/2022)

Canonical (layouts.wiki + oxey.dev/sturdy + AKL archive):
```
v m l c p   x f o u j
s t r d y   . n a e i
z k q g w   b h ' ; ,
```
(Note: an older stored oxey JSON differs slightly; use the oxey.dev page version above.)

**Home row:** L: S T R D Y | R: . N A E I  
(Angle Mod assumed: bottom-left Z K Q G W typed ring/middle/index under angle technique.)

**Design rationale (oxey.dev/sturdy, Oxey's own words):**
- "Very high rolls while at the same time minimizing redirects, and then **bad redirects** (defined as any redirect not involving index) especially."
- Low pinky usage and finger speed.
- Built with self-made analyzer (later Oxeylyzer ecosystem).
- Left index does extra work because left middle column is L/R (low); SFBs `dy`, `cyc`, `dg` cluster near index home so fingers stay local.
- Alternatives considered: home rows `rnts` and `stnd`; rejected for worse index load / rn middle column.
- Known annoyance: **`sk`** feels bad (against finger length; forces left middle curl). Candidate fixes discussed: alt-finger `rk`/`lk`, move to QWERTY-B, etc.
- `n.` common but can be alt-fingered later.
- `ji` (pinky + `.` on index) gives right pinky breathing room.
- `rl` often followed by left-hand keys ("early", "world"); overrepresented in Monkeytype default wordlist.

**Stats (layouts.wiki):** SFB 0.58%, FSB 0.07%, HSB 1.19%, LSB 0.97%, SFS 4.35%, Rolls:Alts 1.85, **Weak redirects 0.16%**. Heavy left ring (10.56% ring usage).

**Community framing (Getreuer / layouts.wiki recs):** "Particularly remarkable in that it has very high rolls while attaining relatively low redirects. Rare combination." High in 2-rolls (alternates hands every other keystroke). Has a **"Whorf index"** (from Whorf 2021): low-usage, high-movement index minimizes SFBs on average but has edge-case bursts ("copying"). Main issue: **K** placement ("task", "milk", "back").

### 1.6 Gallium v2 (Bryson James / GalileoBlues, 2023; v2 ≈ 2025)

**Gallium Rowstag (formerly "Gallium v2") — current recommended (Getreuer updated 2025-12-28; layouts.wiki recs):**
```
b l d c v   j f o u ,
n r t s g   y h a e i
x q m w z   k p ' ; .
```

**Earlier 2023 form (layouts.wiki page; Y/F and P placements differ):**
```
b l d c v   j y o u ,
n r t s g   p h a e i
x q m w z   k f ' ; .
```
README documented post-release tweaks: **X↔Q** swap (QX never occurs; XQ rare) and **J↔Z** swap (lowers SFBs/SFS).

**Home row (current rowstag):** L: N R T S G | R: Y H A E I  
Colstag variant cycles Y, P, F relative to rowstag.

**Lineage / rationale (GitHub README):**
- Inspired by **Nerps** (Smudge); aims to improve comfort/speed.
- Related to **Sturdy** (many Sturdy-descended layouts) and very similar to **Graphite** (Richard Davison, developed independently, weeks earlier/later).
- Goals: break repetitive patterns, balance hand fatigue; **lower rolls / bring alternation:rolls closer together** vs Sturdy-class.
- Vowel-hand **root index letter = H** — large effect on vowel-hand interactions and hand balance.
- Explicitly removes need to alt-finger `SP/PS` (unlike Nerps).
- Pinky/ring load is "necessary evil"; caution for weak pinky/ring users.
- Weaknesses: worse on 450k-word Monkeytype list — SFBs `HY`, `PY`, `PH`, `PF` rise in obscure vocabulary (mitigable by alt-fingering).

**Stats (layouts.wiki 2023 form):** SFB 0.57%, FSB 0.09%, HSB 1.15%, LSB 0.39%, SFS 4.36%, Rolls:Alts 1.42 (more alternation-leaning), Weak redir 0.29%.

**AKL status:** #1 beginner recommendation. Balanced ("decent across the board"). Known issues: right-index sequences in "happy/physics/python"; B-on-pinky-top with R/M ("broad", "thumb").

### 1.7 Semimak (Semi, 2021)

**layouts.wiki / current:**
```
l h v z '   w u o y s
r n t k c   d e a i ;
x j b m q   p g , . /
```
**Home row:** L: R N T K C | R: D E A I ;

**Original (AKL archive / oxey JSON):**
```
f l h v z   q w u o y
s r n t k   c d e a i ;
x ' b m j   p g , . /
```
**Home row (original):** L: S R N T K | R: C D E A I ;

**Semimak-JQ:** same with `'`/`j`/`q` rearranged: `f l h v z ' w u o y / s r n t k c d e a i ; / x j b m q p g , . /`

**Notes:** Created with **genkey** (Oxey's analyzer/generator used to create Semimak — from genkey repo description). Liquids L/R on rings (Canary cites Semimak here). Stats (layouts.wiki): SFB 0.61%, HSB 1.31%, LSB 0.78%, SFS 3.79%, Rolls:Alts 1.36, Weak redir 0.37%. Relatively low rolls (in:out ≈ 0.84 — slightly out-roll leaning on 2-rolls).

### 1.8 Halmak (Nikolay Nemshilov, 2016)

```
w l r b z   ; q u d j
s h n t ,   . a e o i
f m v c /   g p x k y
```

**Home row:** L: S H N T , | R: . A E O I — **comma and period live on home center columns** (first layout to push punctuation into center to de-emphasize indexes).

**Rationale (layouts.wiki):** First "AI-designed" layout claim; genetic/tournament evolution with hand-chosen penalties (vs Klausler/CarpalX empirical timing). Near-50/50 hand balance; de-emphasizes index fingers. Claims "30% better than the best thing." Criticized for **high SFBs and SFTs** vs MTGAP and Colemak-DH.

**Stats:** SFB 2.09% (high for modern), FSB 0.03%, HSB 1.71%, **LSB 0.35%**, SFS 4.53%, Rolls:Alts **1.15** (very alternation-heavy, P4), Weak redir 0.31%. Finger usage quite even (8–12% each).

### 1.9 RSTHD / "rusted" (Simon Zeng / xsznix)

layouts.wiki: "in-roll-focused, **index-heavy** layout. Second layout to put a letter on a **thumb key**, after Maltron. vs Maltron: lower weak redirects, lower pinky usage, removes ol/lo pinky-ring half scissor." ANSI adaptation (2022) translates to standard boards with minimal swaps.

**Community RSTHD (ortho, letter-only; E on thumb/space):**
```
j c y f k   z l o u ;
r s t h d   m n e i a
/ v q g p   x w b , .
```
**Home row (thumbs excluded):** L: R S T H D | R: M N E I A ; **E on thumb**.

**Stats (layouts.wiki, traditional incl. thumb space):** SFB 0.62%, FSB 0.06%, HSB 1.78%, LSB 0.93%, SFS 4.05%, **Rolls:Alts 2.43**, Weak redir 0.24%. Finger usage includes thumbs (9.4% + 17.2% thumb columns in wiki's 10-finger numbering).

### 1.10 MTGAP (Michael Dickens, ~2008–2012; "2.0" famous)

**MTGAP 2.0 (30-key, 2012 blog)** — the version people mean by "MTGAP":
```
, f h d k   j c u l .
o a n t g   m s e r i
q x b p z   y w ' v ;
```
**Home row:** L: O A N T G | R: M S E R I

**Rationale (blog + layouts.wiki):** Simulated annealing optimizer; score 9.67 on author's 8-criteria system. Notable for beating everything before it (arguably except Maltron's thumb-E). Explicitly **repudiates** "most common letters on home row + minimize travel" as sole goals. Own comparison vs Colemak: travel 68069 vs 71162 (better); same-finger both 0.34%; same-hand worse (25.45% vs 21.16%); jump-home 0.02% vs 0.2%; rolls 14.48% vs 6.82%.

**Stats (layouts.wiki 30-key w/ thumbs):** SFB 1.06% (first variant), LSB 0.50%, SFS 4.69%, Rolls:Alts 1.57, 2-roll In:Out ≈ 1.9 (in-roll heavy). Strong middle-finger load (15.82%).

SteveP compare table: MTGAP traditional base effort 1.731, SFB 1.25% (his model), overall 1.787 — between Colemak-DH and Workman.

### 1.11 Nirvana
**Not found** in layouts.wiki catalog, AKL archive (GalileoBlues/Layout-Wiki), O-X-E-Y layouts repo, GitHub search (`nirvana keyboard layout`), or r/KeyboardLayouts search (Reddit API blocked). May be a Discord-private/experimental layout or renamed. No letter map or stats recoverable from public web sources as of this research pass.

### 1.12 Other highly-regarded modern layouts

#### Sind (NotGate, 2021)
```
y , h w f   q k o u x
s i n d c   v t a e r
j . l p b   g m ; / z
```
**Home row:** L: S I N D C | R: V T A E R  
From oxey playground + AKL archive. Listed under Whorf family in AKL archive. Link: notgate.github.io/layout/experimental (page 404 at fetch time).

#### Graphite (Richard Davison, 2023) — Gallium twin
```
b l d w z   ' f o u j ;
n r t s g   y h a e i ,
q x m c v   k p . - /
```
**Home row:** L: N R T S G | R: Y H A E I  
Stats: SFB 0.64%, LSB 0.49%, SFS 4.20%, Rolls:Alts 1.41, Weak redir 0.27%. Differs from Gallium mainly in punctuation (non-standard shift pairs; `.` on different finger). Getreuer: equally recommended; Gallium better for Vim muscle memory (J placement).

#### Pine v4 (Clemenpine / AKL recs)
```
q l c m k   f u o y
n r s t w   p h e a i
j x z g v   b d ' / .   (approx from recommendations grid; v4)
```
Home: L: N R S T W | R: P H E A I  
Similar feel to Gallium/Graphite without PHY/BR/MB issues; main cost: high right-pinky (YI) usage ("hey", "yeah", "playing").

#### Enthium (sunaku, 2026 on wiki; Engram × Hands Down family)
204-key board extraction (thumb alphas):
```
y u o / l   d w z k
b c i e a   , h t n s v
' - = . ;   j m g p f   (+ thumb keys)
```
AKL rec: exceptionally low top-row pinky usage; Vim hjkl cluster; low center-column LSB; costs: high right-pinky SFS (ZSWV — "was"), high weak redirects ("instead", "basically").

#### Night / Dusk / Dusk-WP / Afterburner (thumb-alpha, AKL recs)
34-key class. Night: top recommendation among thumb-alpha; ultra-low SFB/SFS/weak-redir; Whorf index; lateral stretches on Y/P ("you", "people"). Dusk-WP: similar goals, more left-middle load; M laterals ("come", "man").

#### Kuntum / Kuntem (AKL)
Angle-mod-only; "no major problem keys"; high left-pinky usage (T on pinky) with low pinky movement (only V off-home on pinky). Kuntem moves `.` `,` off pinky further.

#### cmini layouts (AKL Discord analyzer culture)
**cmini** = "A layout analyzer in bot form" (Apsu/cmini; also O-X-E-Y/cmini-rs). Discord bot that prints SFB/SFS/scissors/rolls/redirects etc. for a submitted layout. **akl.gg** is a web viewer with engines `mana2` and `cmini`, toggles for **Alts on/off**, spacegrams, rowstag/ortho. Layouts named in akl.gg snippets: `cmini` (val0rance), `crepuscular` (stop.png), `danie`. "Cmini layouts" = layouts tuned/discussed against cmini's metric definitions.

#### Flan / Bcot / Theta / Aster / Noll
**Not found** as named layouts in layouts.wiki, AKL archive, O-X-E-Y stored layouts, or web search results retrieved this session. May be experimental Discord-only layouts or nicknames. Related names that *do* exist: **Boo**, **Siuol**, **Aest** (Smudge), **Anymak**, **Afterburner**, **Aptmak**. If these names came from AKL Discord channels, they likely never got public writeups.

#### Lineage cheat-sheet
```
Colemak → Colemak-DH → (Canary, Rollmak, …)
Nerps (Smudge) → Gallium / Graphite (+ Sturdy influence)
Sturdy (Oxey) → Magic Sturdy, Nitro/Nitro S → Nitrogen/Hydrogen, many
Engram + Hands Down → Enthium
Whorf → "Whorf index" concept used in Sturdy/Night/Dusk
Semimak (genkey) → Rolll → Rollla, Vitrimak, …
```

---

## 2. Alt-fingering & serious analysis metrics

### 2.1 What "alt-fingering" means (AKL Terminology, GalileoBlues/Layout-Wiki)

> Not to be confused with Alternation: **Alt-Fingering** is using a different finger to press a key than traditional touch typing — e.g. QWERTY `b` with the right hand, or `z` with ring instead of pinky.

Traditional map is the 8-home-key + column model; real optimized layouts often **require** non-traditional fingerings (Angle Mod is the canonical example: left bottom row ring/middle/index instead of pinky/ring/middle/index).

### 2.2 How analyzers treat alt-fingering

| Tool | Alt-fingering handling |
|---|---|
| **patorjk KLA** | Fixed per-key finger assignment from the on-screen keyboard config. No dynamic per-bigram finger choice. Scoring weights: distance 33%, finger work 33%, SFB 17%, hand alternation 17% (original). |
| **SteveP fork of KLA** (stevep99.github.io/keyboard-layout-analyzer) | Multiple **keyboard configurations** (finger maps), not per-keypress optimization. Changes vs patorjk: distance→"distance penalty" (Fitts's law, 10° hand approach angle, horizontal×2.0 penalty, finger efforts index 1.0 / middle 1.1 / ring 1.3 / pinky 1.6); reweights to distance 50% / distribution 20% / SFB 30% / **removes hand-alternation element** ("ST and NE are some of the most comfortable bigrams"). |
| **SteveP Keyboard Layout Analysis Tool** (colemakmods analyze.html) | Explicit **keyboard configurations**: Traditional / Ergonomic (Angle Mod) / Alternative / Matrix. Compare page scores each layout under its recommended config. Bigram penalty table: SFB same-row 2.5, top↔mid 3.0, mid↔bot 3.0, top↔bot 4.0; pinky-ring 0.5/1.0/1.5; ring-middle 0.1/0.2/0.3. |
| **layouts.wiki / Keygen Pro** | Stats shown per **Finger Map** variant (Traditional vs Angle Mod columns side-by-side on layout pages). Does not auto-pick best finger per event. |
| **akl.gg / cmini / mana2** | Has explicit **"Alts: on/off"** toggle — recomputes metrics assuming common alternate fingerings. This is the modern AKL-community way to report "SFB under alt fingering." |
| **Oxeylyzer / genkey / cmini-rs / keynergy** | Oxey's family optimizes under a fixed fingering model; alt-finger moves (e.g. Sturdy's `n.`, Gallium's SP/PS) are hand-specified design decisions, not auto. |

**Important consensus (Getreuer):** "It is essential that you use the layout's intended finger placement… Failing to effectively changes the layout's SFBs, likely dramatically for the worse." Alt-fingering is part of the *layout definition* for Angle-Mod layouts, not an optional cheat.

### 2.3 SFB vs SFDR / SFT / same-finger family

- **SFB (same-finger bigram):** two consecutive keys, same finger, different keys. Community ballpark (AKL Terminology; corpus/definition dependent): QWERTY 4.5–6.6%, Dvorak 1.9–2.7%, Colemak 1–1.3%, Workman 2.2–2.9%. Modern AKL designs target **< 1%**, top ones **0.5–0.7%**.
- **SFT / same-finger trigram:** three keys involving a repeated finger (some analyzers).
- **SFS / DSFB (same-finger skipgram / disjoint SFB):** same finger on keys separated by exactly one keystroke ("hay" on QWERTY = h→y skip). Modern layouts often have SFS 3.5–6%. Canary's uneven load concentrates SFS.
- **SFDR / same-finger different-row:** same-finger bigram that also changes row (strictest SFB cost). SteveP penalty table above encodes row-jump severity. layouts.wiki splits SFB into full/half scissors + lateral stretch columns separately rather than one SFDR number.
- **LSB (lateral stretch bigram):** center-column / sideways index motion ("beyond" on QWERTY has be, yo, on). Motivation for Colemak-DH (D/H) and Halmak's center punctuation.

### 2.4 Rolls, alternation, redirects (layouts.wiki "rhythm" + AKL Terminology)

Trigram-level taxonomy (Canary README terminology, matches wiki):

| Category | Definition | Example (QWERTY) |
|---|---|---|
| **Roll** | same-hand sequence continuing one direction, then (definition-dependent) may or may not require a hand switch | `re` out, `er` in |
| **One-hand** | 3+ keys same hand same direction | `wer`, `rew` |
| **Redirect** (roll reversal / pinball) | same-hand trigram that changes direction | `are`, `fad` |
| **Weak redirect / bad redirect** | redirect with **no index and no thumb** (only middle/ring/pinky) | `sad` |
| **Weak-ish redirect** (wiki-only) | redirect involving ≥1 weak finger (ring/pinky) but also index or thumb | `saf` |
| **Alternate** | trigram changing hands twice | `us` + next |
| **Rolls : Alts** | total roll trigrams / total alternation trigrams (1.0 = balanced) | — |

Community % ranges (AKL Terminology, highly corpus-dependent):
| | QWERTY | Dvorak | Colemak | Workman |
|---|---|---|---|---|
| Alternation | 14.2–26.2% | 23.7–44.3% | 15.7–30.1% | 15.3–29.3% |
| Rolls | 20.4–36.9% | 21.5–38.4% | 24.7–45.7% | 23.4–43.3% |
| Redirects | 7.8–13% | 2.3–3.3% | 6.4–10.5% | 6.4–10.3% |

**Design tension:** high rolls usually imply more redirects; Sturdy's fame is high rolls **and** low (especially weak) redirects. Canary claims **>55% of trigrams are rolls**. Gallium deliberately pulls toward alternation (Rolls:Alts 1.42) vs Sturdy (1.85) / Colemak (2.20) / RSTHD (2.43).

**In vs out rolls:** Engram and RSTHD deliberately favor in-rolls (little→index). Canary rejects the dogma that in-rolls are strictly faster ("anecdotal at best"). layouts.wiki reports 2-Roll In:Out and 3-Roll In:Out ratios separately (e.g. Engram 2021 3-roll In:Out 6.44 — extreme).

### 2.5 Scissors (layouts.wiki)

> A scissor is when a finger extends to the far side of the keyboard **at the same time** as an adjacent longer finger flexes to the near side (simultaneous opposing flexion-extension of adjacent fingers).

- **Half scissor (HSB):** 1u distance between the two keys of the scissor bigram.
- **Full scissor (FSB):** 2u distance.
- Skipgram analogues HSS/FSS.
- Severity order: index-middle most tolerable → middle-ring → **ring-pinky least tolerable**.
- Colloquially people lump many awkward motions under "scissors"; serious analysis splits half/full and bigram/skipgram.
- Typical modern targets: total scissor ~1–2.5%; Canary HSB 0.72% is elite; Colemak HSB 1.92% is mediocre by modern standards.

### 2.6 Other metrics the AKL scene tracks

- **Trigram costs / magic keys:** Magic Sturdy etc. use adaptive "magic" keys to consume common trigrams (`N★=ION`, `A★=O`, …). Getreuer documents this.
- **Row jumps:** top↔bottom bigrams cost more (SteveP 4.0 vs 2.5 same-row SFB).
- **Finger speed** (cmini): per-finger velocity proxy from bigram frequency × distance.
- **Spacegrams / thumb space side** (layouts.wiki No Thumbs / Left Space / Right Space columns; akl.gg spacegrams).
- **Whorf index:** pattern of low-usage + high-movement index column that minimizes average SFB but creates burst failures ("copying", "wagyu").

### 2.7 SteveP / KLA historical context

- Old site stevep99.info is **dead (DNS NXDOMAIN)**; content now at **stevep99.github.io** (KLA fork, Seniply, keyboard-tweaks, Keyboard Layout Analysis Tool).
- SteveP's KLA About (partials/about.htm) is the best public critique of patorjk scoring — read for how distance/finger weights/SFB/alternation interact.
- Colemak-DH compare page remains the reference table for Traditional vs Ergonomic (angle) vs Matrix configs (Colemak-DH matrix 1.726 … QWERTY much worse).

---

## 3. Home rows by finger + "high home row" / arched posture

### 3.1 Home rows (traditional columns) of key layouts

| Layout | L pinky | L ring | L middle | L index | L index-str | R index-str | R index | R middle | R ring | R pinky |
|---|---|---|---|---|---|---|---|---|---|---|
| QWERTY | A | S | D | F | G | H | J | K | L | ; |
| Colemak | A | R | S | T | D | H | N | E | I | O |
| Colemak-DH | A | R | S | T | G | M | N | E | I | O |
| Engram 2021 | C | I | E | A | , | . | H | T | N | S |
| Engram 2025 | H | I | A | E | , | . | T | R | S | C |
| Canary (ortho) | C | R | S | T | G | M | N | E | I | A |
| Sturdy | S | T | R | D | Y | . | N | A | E | I |
| Gallium v2 rowstag | N | R | T | S | G | Y | H | A | E | I |
| Graphite | N | R | T | S | G | Y | H | A | E | I |
| Semimak (wiki) | R | N | T | K | C | D | E | A | I | ; |
| Halmak | S | H | N | T | , | . | A | E | O | I |
| RSTHD | R | S | T | H | D | M | N | E | I | A |
| MTGAP 2.0 | O | A | N | T | G | M | S | E | R | I |
| Sind | S | I | N | D | C | V | T | A | E | R |
| Pine v4 | N | R | S | T | W | P | H | E | A | I |

Thumb letters: RSTHD puts **E** on thumb; Night/Dusk-WP/Bunya/Enthium use thumb alphas; MTGAP uses thumbs heavily for space (17.5% in wiki finger table).

### 3.2 Does any layout rest ring/middle on QWERTY W/E (top row)?

**Short answer: no mainstream layout defines home *rest* on the top row.** All catalogued layouts use the middle keyboard row as the home row. However:

1. **Engram explicitly ranks top-row ring/middle (QWERTY W/E positions) as among the most comfortable keys** — "top-center keys (2,3 and 14,15) that allow the longer middle and ring fingers to uncurl upwards." That is why `B Y O U` sit on top and get high-value letters (B=53, Y=59, O=272, U=97 per Norvig thousands in README frequency diagram) instead of being afterthoughts. This is the closest published concept to "high home / arched finger" design: **the top center is a secondary comfort zone for ring/middle**, not a rest position.

2. **SteveP's effort-grid model** (colemakmods model.html) measures natural home positions from Atreus column stagger: middle finger home is **+0.16u above** traditional home row center; index is **−0.32u below** (toward bottom); ring/pinky nearly on home. So biomechanics say middle naturally sits slightly high — but not a full row up to QWERTY W.

3. **"Hands Up" (Oxey)** is named provocatively but its home row is still middle-row `h i e a | d t s r n` ("hiea homerow" started from nrst+hiea with genkey). Not a top-row rest layout.

4. **Canary / Angle Mod** changes *which finger* hits bottom-left keys, not home row height.

5. **Column-stagger / contoured boards** (Atreus, Glove80, Kinesis) physically raise/lower columns per finger length — the ergonomic answer to arching — rather than moving letter assignments to a "high home."

If a user types with ring on W and middle on E as *rest*, that is untrained/individual technique; no optimized layout's published finger map assumes it. Engram's comfort ranking is the design-adjacent idea to investigate further.

---

## 4. English letter-pair trivia for layout design

### 4.1 Letter frequencies
Canonical order (Norvig / Google books; widely cited in layout scene):
**E T A O I N S R H L D C U M F P G W Y B V K X J Q Z**

From a simple unweighted top-10k English word list (type frequency, not corpus-weighted — use only as sanity check):
e 11.6% · i 8.3% · a 8.2% · s 7.7% · r 7.4% · n 7.3% · t 7.2% · o 6.4% · l 4.9% · c 4.6% · d 3.8% · p 3.1% · u 2.9% · m 2.9% · g 2.6% · h 2.2% · b 1.7% · y 1.6% · f 1.4% · v 1.3%

### 4.2 Classic most-common bigrams (corpus-weighted, Mayzner/Norvig tradition)
**th, he, in, er, an, re, on, at, en, nd, ti, es, or, te** — the canonical list every layout optimizes against. Then: ng, ed, ar, nt, al, st, or, io, ra, co, le, ri…

Word-list approximation (intra-word bigrams) ranked: in, er, on, re, ti, es, en, te, at, an, ng, ed, ar, nt, al, st, or, io, ra, co — same neighborhood; **th/he** drop because they're often word-crossing (`t|he`) in a word-split view.

### 4.3 Digraph / roll design implications
- Vowel-vowel hot pairs called out in Canary: **ea, ou, ai, oa, ui, ue** — put on one hand with low SFB/redirect cost.
- Consonant-liquid hell: **L and R combine with nearly every letter**; modern layouts park them on rings and accept mid frequency (Canary, Semimak, Gallium N/R columns).
- Semivowels **W, Y**: high affinity with many partners (`ly ty tw sw`); Canary splits them from the vowel hand.
- Common hard SFBs to engineer around: **ue/oe** (vowel block), **dy/dg** (Sturdy index cluster), **HY PY PH PF** (Gallium on 450k lists), **sk** (Sturdy), **WR** (Canary scissor), **PHY BR MB** (Gallium issues, fixed by Pine).

### 4.4 Q behavior
From top-10k words (unweighted):
- After **q**: **qu ≈ 92%** of following letters (essentially always U).
- Before **q**: **eq ≈ 48%**, aq 13%, iq 11%, sq 7%, cq/nq ~7% each.
- `sq` exists ("square", "squish") but is rare; layouts put Q on left index bottom (Canary) or right pinky (Engram) so `QU` is either cross-hand (Canary: Q left, vowels right — "does not require an awkward stretch on one hand") or same-pinky-with-U-nearby (Engram Q bottom-right pinky, U top-right ring).

### 4.5 Word-final bigrams (top-10k words)
| Suffix | Count | % of words |
|---|---|---|
| -s | 2022 | 20.4% |
| -ed | 563 | 5.7% |
| -ng | 540 | 5.5% |
| -es | 534 | 5.4% |
| -ing | 517 | 5.2% |
| -on | 459 | 4.6% |
| -er | 373 | 3.8% |
| -al | 254 | 2.6% |
| -ns | 240 | 2.4% |
| -rs | 237 | 2.4% |
| -nt | 200 | 2.0% |
| -ly | 189 | 1.9% |

**Design takeaways:** `ng` (as in -ing) and `ed`/`es`/`ts`/`rs` are extremely final-heavy — `N` and `G` should not form a terrible SFB; prefer `G` on index-stretch (Colemak-DH/Canary/Gallium) so `NG` is a comfortable index pair or cross-finger. Final `ER`/`ED`/`TS` reward ring/pinky setups that roll outward or alternate cleanly. Contractions (`'s`, `'t`, `'re`, `'ll`, `'d`) make `'` pair with almost everything except A (Canary's argument for keeping `'` on QWERTY apostrophe spot / near A).

### 4.6 Word-initial bigrams (top-10k)
co, re, in, pr, de, ma, ca, st, di, pa, tr, su, se, ch, ex  
(Classic prose lists also emphasize **th, he, in, re, co, de, di, en, un**.)  
Prefixes `con-/com-/cor-/col-`, `re-`, `pre-`, `dis-`, `sta-/str-` mean initial clusters should not create scissors or weak redirects on the first 2–3 keystrokes of words — matters for "first impression" typing feel.

### 4.7 Practical checklist distilled from sources
1. Separate most vowels onto one hand (except RSTHD/MTGAP styles) to enable rolls; accept and engineer the vowel-vowel SFB set.
2. Keep T/H off the same column (Ian Douglas constraint cited by Getreuer: "no good layout will have T and H on the same column").
3. Park rarest letters (Q, Z, J, X) on hardest keys (outer pinky top/bottom, QWERTY B/Y corners on rowstag).
4. Decide roll-vs-alternation philosophy first (Sturdy/Canary/RSTHD vs Gallium/Halmak/Dvorak).
5. Optimize weak redirects separately from total redirects if targeting >80–100 WPM.
6. Publish Angle-Mod and Traditional finger maps if your bottom-left letters assume reassignment.
7. Test on 10k *and* 450k wordlists (Gallium's documented failure mode).
8. Model `'` `,` `.` explicitly — they participate in `n't`, `'s`, `'re`, sentence cadence.

---

## 5. Downloaded file index

| Path | What |
|---|---|
| `/tmp/opencode/research/html2text.py` | HTML→text converter used throughout |
| `/tmp/opencode/research/notes_alt_layouts.md` | This file |
| `/tmp/opencode/research/keyboard_grids.json` | Parsed key grids (row, col, char) for 15 layouts.wiki pages |
| `/tmp/opencode/research/all_layout_stats.json` | Parsed SFB/scissor/roll/redirect/finger stats |
| `/tmp/opencode/research/extracted/lw_*.md` | Per-layout author/stats/description stubs |
| `pages/lw_2000s_colemak.html|.txt` … (15 layouts) | Full layouts.wiki layout pages |
| `pages/lw_recommendations.html|.txt` | AKL Recommended Layouts guide |
| `pages/lw_terminology.md` | AKL Terminology (SFB, alt-fingering, rolls, redirects…) |
| `pages/lw_akl_layouts.md` | 50+ AKL Discord layout letter maps |
| `pages/ref_reference_metrics_*.txt` | Wiki: rhythm, same-finger, scissors, ngrams, lateral-stretch |
| `pages/gh_engram_readme.md` | binarybottle/engram-2021 full README (scores, criteria) |
| `pages/gh_engram_family.md` | binarybottle/engram family README (2025 Engram-en) |
| `pages/gh_canary_readme.md` | Apsu/Canary full design doc |
| `pages/gh_gallium_readme.md` | GalileoBlues/Gallium README (v2 notes) |
| `pages/gh_graphite_readme.md` | Graphite README |
| `pages/oxey_sturdy.html|.txt` | Oxey's Sturdy design essay |
| `pages/oxey_*.json` | Oxey playground layout strings (sturdy, sind, canary, gallium, semimak) |
| `pages/stevep_kla_about.htm|.txt` | SteveP KLA About (scoring critique) |
| `pages/stevep_klat_readme.md` | SteveP Keyboard Layout Analysis Tool README |
| `pages/colemakmods_compare.txt` | Mod-DH layout comparison table + penalty model |
| `pages/colemakmods_analyze.html` | Analysis tool explanation (configs, finger numbers) |
| `pages/colemakmods_model.html` | Effort grid / finger length / 10° approach model |
| `pages/getreuer_alt_layouts.html|.txt` | Getreuer alt-layouts guide (metrics table, recommendations) |
| `pages/mtgap_blog.html` | MTGAP 2.0 blog post |
| `pages/top10k_words.txt` | google-10000-english for local bigram estimates |

**Failed / blocked fetches:** layouts.wiki wrong URL pattern initially (fixed); stevep99.info DNS dead; akl.gg bot-protection; norvig.com mayzner captcha; patorjk.com 406 on curl; Reddit JSON 403; notgate/semilin/RSTHD personal sites 404; Nirvana/Flan/Bcot/Theta/Aster/Noll not located.

---

## 6. Open questions / next fetches if needed

1. Cyanophage playground metric definitions (getreuer's table source) — for cross-checking SFB%.
2. Norvig Mayzner bigram table raw numbers (site captcha; try wayback or mirror).
3. akl.gg data API (`data.akl.gg`) for alts-on vs alts-off SFB deltas.
4. Hands Down™ Reference full writeup (mentioned as non-AKL recommendation).
5. Confirm Gallium v2 rowstag letter map against latest GalileoBlues/Images SVG (README images not text-extracted).
6. Locate Nirvana / Flan / Bcot / Theta / Aster / Noll inside AKL Discord only (not public web).
