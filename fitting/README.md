# Model refitting

These are development tools for refitting the calibrated pattern model. They
are not used by the runtime commands or the test suite. The `shirolyzer` runtime
is the Rust binary built from this repository; the Python `fitting` and `utils`
modules exist only for these development tools and the corpus parsers.

Verify that the retained tools still import and compile after a code or layout
change:

```bash
python3 -m compileall -q fitting utils corpora
python3 -c "import fitting, utils.fitlib, utils.gateset"
```

Inputs are read from `config/model/`. The shipped coefficients are
in `calibrated.json`; `fit_seed_A.json` is the starting point for
a fresh fit. Outputs are written to `layouts/fitting/`; set
`SHIRO_FITTING_DIR` to use another directory. Intermediate files in that
directory take precedence over the shipped model files, so interrupted runs can
resume. `fit3.py`/`de_polish2.py` produce `pattern_parameters_<TAG>.json`;
`fit2.py` produces `cutting_plane_<TAG>.json`.

Run from the repository root:

```bash
# Fit all active rulings (DISALLOW, GATES, TAG)
python3 -m fitting.fit3 A all Aall

# Global differential-evolution polish (TAG, MAXITER)
python3 -m fitting.de_polish2 Aall 60

# Leave-one-family-out check (DISALLOW, GATES, TAG, PART)
python3 -m fitting.loo3 A all Aall 0

# Compare geometry-only and explicit-gate rulings (TAG, DISALLOW, GATES)
python3 -m fitting.ablate Aall A all
```

`fit2.py` contains the cutting-plane fitting loop and is imported by the other
tools. After a successful fit, review the resulting parameters before copying
them to `config/model/calibrated.json` and regenerating
the baseline.
