# List all available targets
default:
    @just --list

# The tests. `--changed` (what a session runs before it is ready) is the same run: the repository is one crate.
# Warnings are errors, as in CI (its toolchain action sets `-D warnings`): a test config that warns fails here too.
test *flags:
    CARGO_BUILD_WARNINGS=deny cargo nextest run --all-features --no-fail-fast
    CARGO_BUILD_WARNINGS=deny cargo nextest run --no-default-features --features std --no-fail-fast
    CARGO_BUILD_WARNINGS=deny cargo nextest run --no-default-features --features shrink_wrap,quantities --no-fail-fast
    # nextest does not run doctests
    CARGO_BUILD_WARNINGS=deny cargo test --doc --all-features

# Format, sort, spelling, clippy and rustdoc as CI runs them, warnings are errors
lint:
    cargo fmt --check
    cargo sort --check
    typos
    cargo clippy --all-targets -- -D warnings
    cargo clippy --all-targets --all-features -- -D warnings
    cargo clippy --all-targets --no-default-features --features std -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# cargo check the feature sets a firmware and a host use: no unit group, defmt, a few groups, without and with the wire format, CORDIC on an M0
check:
    cargo check --all-features
    cargo check --no-default-features
    cargo check --no-default-features --features=defmt
    cargo check --no-default-features --features=time,information,counting,dimensionless
    cargo check --no-default-features --features=std
    cargo check --no-default-features --features=shrink_wrap,defmt,time
    cargo check --no-default-features --features=electrical,libm --target thumbv7em-none-eabihf
    cargo check --no-default-features --features=angle,dimensionless --target thumbv6m-none-eabi

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

# Collect coverage for the feature sets `just test` uses (shared by cov and cov-html)
_cov-collect:
    cargo llvm-cov clean --workspace
    cargo llvm-cov nextest --no-report --features customary,serde
    cargo llvm-cov nextest --no-report --no-default-features --features std
    cargo llvm-cov nextest --no-report --no-default-features --features shrink_wrap,quantities

# Test coverage as CI computes it: lcov.info plus a summary printed
cov: _cov-collect
    cargo llvm-cov report --lcov --output-path lcov.info
    cargo llvm-cov report --summary-only

# Test coverage as a browsable HTML report
cov-html: _cov-collect
    cargo llvm-cov report --html --open
