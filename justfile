# List all available targets
default:
    @just --list

# The tests. `--changed` (what a session runs before it is ready) is the same run: the repository is one crate.
test *flags:
    cargo nextest run --all-features --no-fail-fast
    cargo nextest run --no-default-features --features std --no-fail-fast
    cargo nextest run --no-default-features --features shrink_wrap,quantities --no-fail-fast
    # nextest does not run doctests
    cargo test --doc --all-features

# Format, sort, spelling, clippy and rustdoc as CI runs them, warnings are errors
lint:
    cargo fmt --check
    cargo sort --check
    typos
    cargo clippy --all-targets -- -D warnings
    cargo clippy --all-targets --all-features -- -D warnings
    cargo clippy --all-targets --no-default-features --features std -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# cargo check the feature sets a firmware and a host use: no unit group, defmt, a few groups, without and with the wire format
check:
    cargo check --all-features
    cargo check --no-default-features
    cargo check --no-default-features --features=defmt
    cargo check --no-default-features --features=time,information,counting,dimensionless
    cargo check --no-default-features --features=std
    cargo check --no-default-features --features=shrink_wrap,defmt,time
    cargo check --no-default-features --features=electrical,libm --target thumbv7em-none-eabihf

# A library has nothing to put on PATH: builds it with the deploy profile, as a dependent's release build would
install:
    cargo build --profile deploy --locked --all-features

# Bring this checkout up to main: refuses on local changes, pulls, builds
deploy:
    #!/usr/bin/env bash
    set -euo pipefail
    dirty=$(git status --porcelain)
    if [ -n "$dirty" ]; then echo "local changes, not deploying:"; echo "$dirty"; exit 1; fi
    git pull --ff-only
    just install

# Regenerate the tables of docs/customary.md from the sourced catalogue of customary units
customary-docs:
    CUSTOMARY_DOCS=update cargo nextest run --all-features docs_page_tables
