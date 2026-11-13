# Common-word review of the best layout

Layout reviewed: `ftrhcguenjsvld'oawpimzxky,.bq/`

Word list: HermitDave `en_50k.txt`, filtered to alphabetic words of length at
least two, top 10,000 by frequency. Total weighted tokens: 628,837,185.

Command:

```sh
shirolyzer/target/release/shirolyzer words \
  "ftrhcguenjsvld'oawpimzxky,.bq/" \
  --wordlist /tmp/opencode/en_50k.txt --limit 10000 --top 20
```

## Aggregate

| metric | value |
| --- | ---: |
| weighted average bigram cost | 2.6913 |
| weighted average trigram cost | 4.4463 |
| true SFB | 0.00% of trigrams |
| true SFT | 0.00% of trigrams |
| true SFS | 2.15% of trigrams |
| onehand | 2.09% of trigrams |
| redirect | 3.58% of trigrams |
| repeat | 8.05% of trigrams |
| words with SFS | 4.07% of tokens |
| words with onehand | 3.92% of tokens |
| words with redirect | 6.43% of tokens |
| words with repeat | 9.43% of tokens |

The true same-finger rates are low, but the common-word view exposes a set of
high-cost same-hand stretches that the corpus-wide averages hide.

## High-cost common bigrams

| bigram | fingers | physical keys | cost | common contexts |
| --- | --- | --- | ---: | --- |
| `we` | RI-RM | `K I` | 5.713 | `we`, `well`, `new`, `few`, `week`, `between`, `power` |
| `ew` | RM-RI | `I K` | 5.713 | same words as `we` |
| `ty` | LR-LI | `W B` | 5.700 | `-ty`, `-ity`: `city`, `pretty`, `party`, `anything`, `everything` |
| `dg` | LM-LI | `F Y` | 7.234 | `edge`, `judge`, `bridge`, `knowledge`, `budget` |
| `tm` | LR-LI | `W Z` | 6.956 | `christmas`, `apartment`, `department`, `treatment`, `investment` |
| `sf` | LP-LR | `A Q` | 6.554 | `successful`, `transfer` |
| `ms` | LI-LM | `Z A` | 6.497 | `-ms` endings, `some`, `same` |
| `sm` | LP-LR | `A Z` | 6.357 | `sm-` words, `some`, `same` |
| `rl` | LM-LI | `E D` | 5.236 | `girl`, `world` |
| `ck` | LI-LM | `T V` | 4.923 | `-ck` words |
| `my` | LR-LI | `Z B` | 4.283 | `my`, `myself` |

`we` is the clearest problem: it is one of the most frequent words in the list
(6,762,038 weighted tokens) and the model charges the right-index/right-middle
cross-row pair 5.713. `ty` is the next practical target because it recurs in a
large family of `-ty`/`-ity` words rather than in one high-frequency token.

## Pattern-heavy common words

Frequency-weighted counts from the word list:

- SFS: `when`, `where`, `had`, `god`, `must`, `being`, `happened`, `same`,
  `most`, `miss`, `myself`, `open`.
- Onehand: `right`, `people`, `thought`, `night`, `new`, `tonight`, `around`,
  `fine`, `being`, `world`, `might`, `found`.
- Redirect: `about`, `want`, `would`, `some`, `everything`, `something`,
  `mean`, `happened`, `first`, `anything`, `again`, `wait`.
- Repeat: `all`, `good`, `look`, `been`, `gonna`, `really`, `well`, `need`,
  `will`, `see`, `little`, `sorry`.

The SFS list is especially relevant because these are not rare dictionary
entries: `when`, `where`, and `had` are all above one million weighted tokens.
The onehand list is dominated by `right`, `people`, `thought`, and `night`.

## Optimization experiment

`words --out-corpus` can turn the same weighted word list into a parsed corpus:

```sh
shirolyzer/target/release/shirolyzer words \
  "ftrhcguenjsvld'oawpimzxky,.bq/" \
  --wordlist /tmp/opencode/en_50k.txt --limit 10000 \
  --out-corpus /tmp/opencode/common-words-corpus
```

A short run with the common-word corpus as the **secondary** corpus kept the
best layout unchanged:

```sh
shirolyzer/target/release/shirolyzer generate \
  --seed-layout "ftrhcguenjsvld'oawpimzxky,.bq/" \
  --secondary-corpus /tmp/opencode/common-words-corpus \
  --runs 4 --iters 50000 --ils 2 --secondary-rounds 2 \
  --polish-rounds 2 --seed 1
```

A short run with the common-word corpus as the **primary** corpus found
`cthrmguenwskdlyoajpivfxqz,.b'/` (common-word primary 4.3760). It improved the
common-word weighted averages to bigram 2.6394 and trigram 4.2877 and reduced
redirects to 3.09%, but it increased SFS to 2.33% and onehand to 2.31% and
scored 15,462 on the shipped primary corpus. The word list has no spaces or
punctuation, so a common-word-only objective is not a viable replacement for
the shipped primary corpus.

## Weighted-blend experiment

The common-word corpus can be merged with the shipped `shai` corpus instead of
replacing it. Merging at 10/15/20/25/30/50% common-word letter weight (the
remaining share is `shai`) and passing the result to
`generate --primary-corpus /tmp/opencode/blend-N` produced two letter
arrangements:

| blend | layout | shai primary | en50k bigram | en50k SFS | en50k onehand | en50k redirect |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| shipped best | `ftrhcguenjsvld'oawpimzxky,.bq/` | 4.5068 | 2.6913 | 2.15% | 2.09% | 3.58% |
| 10–20% | `mhrcvguenwtdls,oajpifxzyk./bq'` | 4.5447 | 2.6733 | 2.62% | 1.96% | 3.43% |
| 20–50% | `vhrcsmuenwtdlg,oajpifxzyk./bq'` | 4.5379 | 2.6969 | 1.82% | 2.06% | 3.36% |

Both candidates cut the targeted stretches: `we` `5.713 → 2.581`, `ty`
`5.700 → 3.825`, `dg` `7.234 → 2.9–3.6`, `ms`/`sm` `6.5/6.4 → 2.9`, and
`tm` `6.956 → 4.0` on the stronger candidate. The gain is not free, though.
The mildest candidate (`mhrc`) costs `+0.84%` on the shipped primary score and
raises common-word SFS from `2.15%` to `2.62%`. The stronger `vhrc` candidate
costs `+0.69%` primary and lowers SFS/redirects, but raises raw travel on
independent word lists: on `count_1w` and Google's 10k list its weighted bigram
cost is `2.7405`/`2.7677` versus `2.7033`/`2.7205` for the shipped best.

The blend also moves the discomfort rather than removing it. Both candidates
put `w` and `i` on a right pinky-ring reach: `wi` costs `6.602` over 1.56M
weighted tokens (about `10.3M` weighted), versus `3.549` over the same tokens
on the shipped best (about `5.5M`). That single regression consumes most of the
`we`/`ew` saving (`9.84M → 4.49M`), so the net change across the targeted
bigrams is small and comes with a worse primary score.

## Weakness stress test

Ranking the `analyze` worst lists by `cost × corpus count` (rather than raw
cost) puts the practical weaknesses in this order:

| pattern | fingers | cost | count | weighted cost |
| --- | --- | ---: | ---: | ---: |
| `we` | RI-RM | 5.713 | 127,394,400 | 727.8M |
| `whe` | RM-LM-RM | 11.350 | 38,653,800 | 438.7M |
| `som` | LR-LI-LM | 10.855 | 29,747,600 | 322.9M |
| `ien` | RP-RM-RR | 13.950 | 22,734,800 | 317.2M |
| `hey` | LM-RM-LI | 9.101 | 34,037,300 | 309.8M |
| `tiv` | LR-RP-LR | 11.600 | 26,066,600 | 302.4M |
| `tim` | LR-RP-LP | 9.309 | 32,254,800 | 300.3M |

`we` is the largest high-cost bigram and `whe` is the largest high-cost
trigram. The `whe` cost comes from `w` and `e` sharing the right-middle finger
in the chosen fingering (`when`, `where`, `whether`, `while`). `tiv` is an SFS
(`t` and `v` both on LR); `som` and `ien` are redirects.

A complete single-letter-swap search was run over all 325 letter pairs. After
applying the configured hand restriction (`z, x, c, v, t, y` must stay on the
left), 253 swaps remain valid. Each was scored with `analyze` primary plus the
generator's finger-balance penalty (per-hand targets P18/R25/M28/I29), which
reproduces the `generate` primary of `4.555126` for the shipped best. No valid
single swap improves that composite score; the closest are `jq` (`+0.00104`)
and `vm` (`+0.00113`).

The only valid swap that both reduces the targeted weakness score and stays
within `+0.002` composite is `vm`:

```text
ftrhcguenjsmld'oawpivzxky,.bq/
```

It swaps `v` and `m` (both stay on the left hand). It lowers the weakness score
by `4.4M` (`-2.1%`), lowers `analyze` primary from `4.5068` to `4.4803`, and
improves bigram/trigram cost and redirects on all three word lists, but raises
common-word SFS slightly (`en50k 2.15% → 2.26%`, `count_1w 2.90% → 3.07%`,
`google10k 3.21% → 3.37%`) and worsens the generator's finger-balance term by
about the same amount that it improves raw pattern cost.

The direct fixes for `whe`/`we` are worse trade-offs. `wy`, `jy`, and `zq` cut
`whe`/`we` but violate the hand restriction. `jw`
(`ftrhcguenwsvld'oajpimzxky,.bq/`) is valid and cuts `whe` `11.35 → 4.01` and
`we` `5.713 → 2.58`, but raises `wi` `3.55 → 6.60` and worsens `count_1w` and
`google10k` SFS/redirects. Adding `ai` to `jw`
(`ftrhcguenwsvld'oijpamzxky,.bq/`) cuts the weakness score by `15.9M`, but
raises `en50k` bigram cost to `2.7171` and SFS to `3.07%`, so it is not a
common-word improvement.

The current layout is therefore a strong local optimum under the shipped
objective, hand restrictions, and balance guardrail. The remaining `whe`/`we`
weakness is a structural trade-off: moving `w` off the right-middle finger
removes it, but every tested alternative either creates a worse `wi`/pinky-ring
reach or raises overall cost. If raw pattern cost and common-word averages are preferred over the balance
term, `vm` is a reasonable optional variant; it is not a strict improvement
over the shipped best, and it is only acceptable if the ring-ring SFS trade-off
below is acceptable.

### SFS finger-pair check

The `vm` swap's SFS increase is not index-middle. Approximate `en_50k` SFS
composition using the chosen-fingering guide contexts:

| SFS pair | base | `vm` | change |
| --- | ---: | ---: | ---: |
| RM-RM (middle-middle) | 30.6% | 28.8% | -1.8 |
| LM-LM (middle-middle) | 14.2% | 13.3% | -0.9 |
| LI-LI (index-index) | 13.5% | 12.9% | -0.6 |
| RI-RI (index-index) | 4.7% | 4.7% | 0.0 |
| LP-LP (pinky-pinky) | 18.6% | 5.2% | -13.4 |
| RR-RR (ring-ring) | 13.0% | 12.4% | -0.6 |
| LR-LR (ring-ring) | 3.6% | 20.4% | +16.8 |
| RP-RP (pinky-pinky) | 1.8% | 2.3% | +0.5 |
| **index/middle total** | **63.0%** | **59.8%** | **-3.2** |
| **ring/pinky total** | **37.0%** | **40.2%** | **+3.2** |

The extra SFS is `LR-LR` (ring-ring), driven by `time`, `something`, and
`mother`; the `LP-LP` SFS in `must`/`same`/`most`/`miss` drops instead. So `vm`
trades pinky-pinky SFS for ring-ring SFS, not for index-middle SFS. Under the
"index-middle only" condition, `vm` is rejected.

Other variants do not resolve the trade-off. `ai` lowers ring/pinky SFS to
29.9% but worsens `en_50k` bigram/trigram (`2.7136`/`4.4898`) and redirects
(`3.73%`). `lm` lowers ring/pinky SFS to 33.1% and redirects to 3.48% but
worsens bigram/trigram (`2.7224`/`4.5667`). `vm+ai` lowers ring/pinky SFS to
32.7% and redirects to 3.50%, but bigram/trigram are still worse than base
(`2.7032`/`4.4571`). `vm+ky` keeps the bigram/trigram/redirect gains
(`2.6793`/`4.4262`/`3.37%`) but raises ring/pinky SFS to 39.1%. No tested
variant improves bigram/trigram/redirects while keeping the extra SFS
index-middle.

### Frequency versus cost

The strengths are on the most common motions:

- Top 10 bigrams by count: 19.9% of bigram occurrences, average cost `2.39`
  versus `2.69` overall.
- Top 10 trigrams by count: 9.6% of trigram occurrences, average cost `3.69`
  versus `4.51` overall.
- Top 100 trigrams: 30.7% of occurrences, average cost `3.76`.
- Alternation covers 66.5% of bigrams; rolls cover 48.5% of trigrams
  (`inroll` 25.8%, `outroll` 22.7%). These are the comfortable categories.

The weaknesses are on less common motions:

- High-cost bigrams (`cost > 6`): 0.34% of bigram occurrences, 0.84% of
  weighted bigram cost.
- High-cost trigrams (`cost > 6`): 8.9% of trigram occurrences, 19.5% of
  weighted trigram cost; `cost > 8`: 6.0% of occurrences, 15.0% of weighted
  cost.
- The named high-cost trigrams (`whe`, `som`, `ien`, `hey`, `tiv`, `tim`) each
  contribute roughly 0.3–0.4% of total weighted trigram cost; `whe` is 0.16% of
  trigram occurrences and `we` is 0.39% of bigrams.

So the answer is yes, with one caveat: the dominant motions are the strengths,
and the biggest named weaknesses are much rarer than the dominant ngrams. But
the high-cost trigram tail is not negligible — about 9% of occurrences and 20%
of weighted cost — so the weaknesses are concentrated in a meaningful minority
of less common trigram patterns, not in vanishingly rare motions.

## Conclusion

The best layout is good on corpus-wide SFB/SFT, but common conversational words
still concentrate discomfort in a few same-hand stretches, especially `we`/`ew`
and the `-ty`/`-ity` family. The new `words` command makes those costs visible
and can emit a weighted common-word corpus for generation. The weighted-blend
experiment shows that a common-word objective can reduce the named stretches,
but it raises the primary score and shifts cost onto other frequent bigrams
(`wi`) and independent word lists. The valid single-swap stress test found no
strict improvement; `vm` is the only low-cost trade-off, and it gives up some
finger balance for lower pattern cost, but it is rejected under the
index-middle-only SFS condition because its extra SFS is ring-ring. The shipped
best layout remains the best
compromise under the calibrated objective, and a generator-side common-word
penalty is not justified by the current evidence; if one is added, it needs a
weight sweep and validation on several frequency lists rather than the single
subtitle list used here.
