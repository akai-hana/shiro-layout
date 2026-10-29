# Common shirolyzer tasks. Override the interpreter with:
#   just PYTHON=python3 test
python := env_var_or_default("PYTHON", ".venv/bin/python")

# List available recipes
default:
    @just --list

# Search for a layout using the current configuration
compute *ARGS:
    {{python}} shirolyzer/compute.py {{ARGS}}

# Analyze LAYOUT, or the configured current layout when omitted
analyze layout="" *ARGS:
    {{python}} shirolyzer/analyze.py "{{layout}}" {{ARGS}}

# Render LAYOUT as a finger-annotated ASCII grid
render layout *ARGS:
    {{python}} -m shirolyzer render "{{layout}}" {{ARGS}}

# Write LAYOUT to an XKB symbols file
xkb layout *ARGS:
    {{python}} -m shirolyzer xkb --layout "{{layout}}" {{ARGS}}

# Rank the reference layouts
compare *ARGS:
    {{python}} -m shirolyzer compare {{ARGS}}

# Run questionnaire probes and model rulings
audit *ARGS:
    {{python}} -m shirolyzer audit {{ARGS}}

# Run the unit and regression tests
test:
    {{python}} -m unittest discover -s shirolyzer/tests -q

# Verify references and the shipped-model baseline
baseline:
    {{python}} -m shirolyzer check-baseline

# Run the full local verification suite
check: test baseline
