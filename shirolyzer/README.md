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

## Analysis
> Details about shirolyzer's layout generation

The **shirolyzer** also provides an analyzer, to evaluate layouts under the same criteria that the generator tries to optimize for.

> TODO: explain more in detail

---

<div align="center">

</> with <3 by [akai_hana](https://github.com/akai-hana)

</div>
