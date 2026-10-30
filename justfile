# Common shirolyzer tasks. The Rust binary is the only runtime implementation.
# Build it once with `just build`; the Python tree is development tooling only.
shirolyzer := "shirolyzer/target/release/shirolyzer"
manifest := "shirolyzer/Cargo.toml"
python := env_var_or_default("PYTHON", ".venv/bin/python")

# List available recipes
default:
    @just --list

# Build the Rust binary in release mode
build:
    cargo build --manifest-path {{manifest}} --release

# Search for a layout using the current configuration
compute *ARGS: build
    {{shirolyzer}} compute {{ARGS}}

# Analyze LAYOUT, or the configured current layout when omitted
analyze layout="" *ARGS: build
    {{shirolyzer}} analyze "{{layout}}" {{ARGS}}

# Render LAYOUT as a finger-annotated ASCII grid
render layout *ARGS: build
    {{shirolyzer}} render "{{layout}}" {{ARGS}}

# Write LAYOUT to an XKB symbols file
xkb layout *ARGS: build
    {{shirolyzer}} xkb --layout "{{layout}}" {{ARGS}}

# Rank the reference layouts
compare *ARGS: build
    {{shirolyzer}} compare {{ARGS}}

# Run questionnaire probes and model rulings
audit *ARGS: build
    {{shirolyzer}} audit {{ARGS}}

# Run the Rust unit, parity, and command tests
test:
    cargo test --manifest-path {{manifest}} --release

# Verify references and the shipped-model baseline with the Rust binary
baseline: build
    {{shirolyzer}} check-baseline

# Verify the retained Python development tools still import
dev-tools:
    {{python}} -m compileall -q shirolyzer corpora
    {{python}} -c "import shirolyzer.fitting, shirolyzer.utils.fitlib, shirolyzer.utils.gateset"

# Run the full local verification suite
check: test baseline
    cargo fmt --manifest-path {{manifest}} --check
    cargo clippy --manifest-path {{manifest}} --all-targets -- -D warnings
