# Verification helpers. The Rust binary is the only runtime implementation.
# Build it once with `just build`; the Python tree is development tooling only.
shirolyzer := "target/release/shirolyzer"
manifest := "Cargo.toml"
python := env_var_or_default("PYTHON", ".venv/bin/python")

# List available recipes
default:
    @just --list

# Build the Rust binary in release mode
build:
    cargo build --manifest-path {{manifest}} --release

# Run the Rust unit, parity, and command tests
test:
    cargo test --manifest-path {{manifest}} --release

# Measure generate run-to-run variance across seeds
variance *ARGS: build
    {{python}} utils/compute_variance.py {{ARGS}}

# Sweep config variants across seeds and report preference metrics
sweep *ARGS: build
    {{python}} utils/preference_sweep.py {{ARGS}}

# Verify the retained Python development tools still import
dev-tools:
    {{python}} -m compileall -q fitting utils corpora
    {{python}} -c "import fitting, utils.fitlib, utils.gateset"

# Run the full local verification suite
check: test
    cargo fmt --manifest-path {{manifest}} --check
    cargo clippy --manifest-path {{manifest}} --all-targets -- -D warnings
