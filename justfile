# Common shirolyzer tasks. The Rust binary is the primary implementation.
# Build it once with `just build`; the Python tree remains as a parity oracle.
shirolyzer := "rust/target/release/shirolyzer"
python := env_var_or_default("PYTHON", ".venv/bin/python")

# List available recipes
default:
    @just --list

# Build the Rust binary in release mode
build:
    cargo build --manifest-path rust/Cargo.toml --release

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
    cargo test --manifest-path rust/Cargo.toml --release

# Verify references and the shipped-model baseline with the Rust binary
baseline: build
    {{shirolyzer}} check-baseline

# Compare the Rust binary against the frozen Python oracle
parity: build
    {{python}} -m shirolyzer check-baseline
    {{shirolyzer}} check-baseline

# Run the full local verification suite
check: test parity
    cargo fmt --manifest-path rust/Cargo.toml --check
    cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings

# Run the Python test suite as the frozen reference implementation
py-test:
    {{python}} -m unittest discover -s shirolyzer/tests -q
