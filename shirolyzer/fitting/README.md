# Model refitting

These are development tools for refitting the calibrated pattern model. They
are not used by the runtime commands or the test suite.

Inputs are read from `shirolyzer/config/model/`. Outputs are written to
`shirolyzer/results/fitting/`; set `SHIRO_FITTING_DIR` to use another
directory. Intermediate files in that directory take precedence over the
shipped model files, so interrupted runs can resume.

Run from the repository root:

```bash
# Fit all active rulings (DISALLOW, GATES, TAG)
python3 -m shirolyzer.fitting.fit3 A all Aall

# Global differential-evolution polish (TAG, MAXITER)
python3 -m shirolyzer.fitting.de_polish2 Aall 60

# Leave-one-family-out check (DISALLOW, GATES, TAG, PART)
python3 -m shirolyzer.fitting.loo3 A all Aall 0

# Compare geometry-only and explicit-gate rulings (TAG, DISALLOW, GATES)
python3 -m shirolyzer.fitting.ablate Aall A all
```

`fit2.py` contains the cutting-plane fitting loop and is imported by the other
tools. After a successful fit, review the resulting theta before copying it to
`shirolyzer/config/model/theta_geo_A_all.json` and regenerating the baseline.
