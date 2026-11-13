# shiro【白】

Shiro is an ANSI alternate keyboard layout designed around context-dependent
fingering. It was generated with **shirolyzer**, the analyzer/generator in this
repository.

## Layout

```text
f t r h c  g u e n j
 s v l d '  o a w p i
  m z x k y  , . b q /
```

30-character string:

```text
ftrhcguenjsvld'oawpimzxky,.bq/
```

Reference name: `shiro-e-rest`. Original seed: `382233130`.

## Main focuses

- **English-first, Spanish-aware.** The primary corpus is Shai; Douglas is the
  secondary corpus, so the layout is tuned for English without ignoring Spanish.
- **Context-dependent fingering.** The layout assumes the configured alternate
  fingerings, not a fixed one-finger-per-column model. This is how it reaches
  zero conventional SFBs and keeps rolls/redirects under control.
- **Rest-key aware.** The calibrated model includes a `rest_penalty` of `0.2`
  so frequent letters prefer the geometry's rest keys.
- **One-handed shortcut constraint.** `z x c v t y` are restricted to the left
  hand so common Ctrl shortcuts remain one-handed.
- **Finger balance.** The generator targets per-hand pinky/ring/middle/index
  loads of `18 / 25 / 28 / 29%`, rather than treating all fingers equally.
- **SFS quality gate.** Additional SFS are acceptable only when they involve
  index/middle fingers, not ring/pinky fingers.

## Decisions taken

- **Kept the base layout over `vm`.** `vm` (swap `m`/`v`) improves the common
  bigram/trigram/redirect averages, but its extra SFS is `LR-LR` (ring-ring):
  `time`, `something`, and `mother` become same-ring-finger trigrams. The SFS
  mix goes from `63.0/37.0` index-middle/ring-pinky to `59.8/40.2`, so it fails
  the SFS condition.
- **Rejected `ai` and `lm`.** They improve the SFS finger-class mix but pay too
  much on common letter patterns: `ai` hurts `ea`, `wa`, `at`, and `in`; `lm`
  hurts `ll`, `ls`, `lt`, `les`, and `mor`.
- **No valid single-letter swap improves the shipped composite.** The closest
  valid swaps are `jq` and `vm`; both are small trade-offs, not strict wins.
- **The base is a local optimum.** Seeding `generate` from `vm` or `jw` quenches
  back to the base layout under the shipped objective and hand restrictions.

## Scores

| metric | value |
|---|---:|
| primary (Shai) | `4.506751260` |
| secondary (Douglas) | `4.658232666` |
| generator composite | `4.555126` |
| common-word bigram cost (en50k) | `2.6913` |
| common-word trigram cost (en50k) | `4.4463` |
| common-word SFS (en50k) | `2.15%` |
| common-word one-hand (en50k) | `2.00%` |
| common-word redirect (en50k) | `3.58%` |

The biggest weighted weaknesses are `we`, `whe`, `som`, `ien`, `hey`, `tiv`,
and `tim`. Most of the cost is on uncommon motions: high-cost bigrams
(`cost > 6`) are only `0.34%` of occurrences, and the top 10 trigrams average
`3.69` versus `4.51` overall. The high-cost trigram tail is still meaningful
(`8.9%` of occurrences, `19.5%` of weighted cost), so it was not ignored.

## Reproduce

The exact config snapshot used for this layout is
[`config.yaml.bak`](config.yaml.bak). The layout string is in
[`layout.txt`](layout.txt).

Analyze the saved layout with the saved config:

```sh
target/release/shirolyzer analyze "$(cat layouts/shiro/layout.txt)" \
  --config layouts/shiro/config.yaml.bak
```

Generate a new layout into a per-layout directory:

```sh
target/release/shirolyzer generate --layout shiro --seed 382233130
```

`generate --layout <name>` creates `layouts/<name>/` and writes the layout,
config snapshot, generation record, analysis, README, and XKB files there.

## XKB

The generated symbols are in [`xkb/shiro`](xkb/shiro). Install them on the
current X display with:

```sh
layouts/shiro/xkb/install.sh
```

## Files

- `layout.txt` — final 30-character layout.
- `config.yaml.bak` — exact shirolyzer config snapshot.
- `xkb/shiro` — XKB symbols file.
- `xkb/install.sh` — per-layout XKB installer.
- `analysis.txt`, `analysis.json` — generated layout analysis.
- `generate.txt`, `generate.json` — generation record when created with
  `generate --layout shiro`.

For the full investigation behind these decisions, see
[`research/findings/common_words_best_layout.md`](../../research/findings/common_words_best_layout.md).
