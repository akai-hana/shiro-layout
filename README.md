<div align="center">

# shiro【白】

###### An alternate keyboard layout that's highly adaptive and context-dependent.

<!-- (image) -->

</div>

---

hi hello

come back later for a more awesome readme

i just made this repo like a couple days ago at most when you're reading this.

## Installing a generated layout (X11)

Generate an XKB symbols file from any 30-character layout:

```bash
python3 shirolyzer/xkb.py --layout ";aiojvlnrwhfpeygtcmsbxquzkd,./" --name cfgE
```

Then install it for the current display:

```bash
xkb/install.sh cfgE
```

`xkb/install.sh` with no arguments installs the shipped `xkb/mine` file as
`mine`.  `compute.py --xkb` writes the selected layout to a generated safe name
under `xkb/`; use `--xkb-name` to choose a name.  `xkb/mine` is only
overwritten when `--xkb-name mine` is passed explicitly.

---

<div align="center">

</> with <3 by [akai_hana](https://github.com/akai-hana)

</div>
