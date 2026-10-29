<div align="center">

# shirolyzer【白ライザー】

###### Layout analyzer/generator based on preferential, context-dependent fingering. Initially built for the Shiro layout.

<!-- (image) -->

</div>

---

### Quick anchors

- [a](#a)
    - [b](#b)

---

# Introduction

## About 白ライザー

The **shirolyzer** is an alt keyboard layout analyzer and generator, initially built for the purpose of iterating generations/analysis in order to reach shiro's final key arrangement.

## Generation
> shirolyzer's layout generation capabilities and specifications

### Search effort

`compute.py` defaults to 28 random restarts in addition to the layouts loaded
from `config/references.toml`, 100000 anneal iterations per run, and 8 ILS
rounds per top layout.  These values are calibrated in
`results/calibration/REPORT.md`: the primary score plateaus around 40k
iterations and ten total seeds, while additional ILS rounds keep improving the
best result.  Override them per run with `--runs`, `--iters`, and `--ils`.

`--time-budget` is optional.  It accepts seconds or `30m`-style strings and
stops between runs/ILS rounds, always saving the best result found so far.

### Diversity-aware results

Near-equal results are clustered by score and by key-position distance before a
layout is selected.  A candidate is valid when its primary score is no more
than `--valid-tolerance` worse than the best primary result and, when both have
a secondary score, its secondary score is no more than
`--secondary-tolerance` worse.  Only layouts at least
`--min-layout-distance` key positions apart are kept, up to `--max-valid`
layouts.

If one valid layout remains it is reported as the clear winner.  Otherwise one
layout is still selected for `--xkb`, but every valid layout is printed and
written to the JSON `valid_layouts` list, and each is appended to the reference
file (or `--append-references`) under a unique name.

`--polish-rounds N` runs additional quench/anneal passes over the top
`--polish-candidates` distinct layouts until a full pass improves none, or the
round limit is reached.  Polishing is disabled by default; enable it when the
near-equal alternatives should be optimized further before selection.

### Corpora

On layout generation, it is capable of taking corpora under different weightings and weightings of priority:

- **Primary:** It takes one or more primary corpora at the same time, to be evaluated at the same time ("a priori") in a blend of varying weights
           (e.g. 100, 50:50, 50:25:25, 80:20:20...).

- **Secondary:** It can optionally take any amount of corpora to be evaluated _after_ an initial generation, ("a posteriori").
    > After generating a layout based off of primary corpus/corpora, it will optimize for secondary corpus/corpora where it doesn't hurt primary,
    > under different levels of "primary sacrifice": to what degree can primary optimization be harmed to benefit secondary (e.g. 1%, 0.1%, 5%, 50%...).

### Alt-fingers & Context

The generator can take user-defined alt-fingers into consideration as criteria to optimize for. \
These will override the selected fingermap.

Defined alts are based off of physical key positions, instead of characters. \
This allows the generator to take them into consideration on the letter arrangement generated, instead of not being accounted and optimized for.

Alts can depend on wide rules (e.g. _"no full reverse scissors with fingers M-L-P"_), or on specific context examples (finger-positional bigrams/trigrams).

Not all defined alt-fingers are equal: they 

<!--TODO: make sacrifice be defined on a percentage basis alongside current numerical basis-->

<!--TODO: make it so that sacrifice can be defined under specific parameters: % of rolls, % of SFBs, % of redirects...-->

### XKB output

`compute.py --xkb` writes the selected layout to `xkb/<name>`, where the default
name is a safe hash-derived name.  Use `--xkb-name` to choose a name; the
shipped `xkb/mine` is only overwritten when `--xkb-name mine` is passed
explicitly.

To convert an existing layout without running the generator:

```bash
python3 shirolyzer/xkb.py --layout ";aiojvlnrwhfpeygtcmsbxquzkd,./" --name cfgE
python3 shirolyzer/xkb.py --layout "$(cat layout.txt)" --out xkb/shiro --install
```

Install a generated symbols file (or the default `xkb/mine`) with:

```bash
xkb/install.sh            # installs xkb/mine as "mine"
xkb/install.sh cfgE       # installs xkb/cfgE
xkb/install.sh FILE NAME  # installs an arbitrary symbols file under NAME
```

The generated symbols use the same punctuation mapping as `xkb/mine`, keep the
apostrophe on both `<AC11>` and the ISO `<LSGT>` key, and can be compiled with
`xkbcomp` against an Xvfb display for a dry-run check.

## Analysis
> Details about shirolyzer's layout generation

The **shirolyzer** also provides an analyzer, to evaluate layouts under the same criteria that the generator tries to optimize for.

> TODO: explain more in detail

---

<div align="center">

</> with <3 by [akai_hana](https://github.com/akai-hana)

</div>
