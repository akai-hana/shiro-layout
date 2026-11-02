# Verification helpers. The Rust binary is the only runtime implementation.
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

# Run the Rust unit, parity, and command tests
test:
    cargo test --manifest-path {{manifest}} --release

# Verify the retained Python development tools still import
dev-tools:
    {{python}} -m compileall -q shirolyzer corpora
    {{python}} -c "import shirolyzer.fitting, shirolyzer.utils.fitlib, shirolyzer.utils.gateset"

# Run the full local verification suite
check: test
    cargo fmt --manifest-path {{manifest}} --check
    cargo clippy --manifest-path {{manifest}} --all-targets -- -D warnings
