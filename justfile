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

# Export the logo from assets/logo/logo.typ: SVG of each variant, PNG files at fixed sizes. Needs typst 0.13 or
# newer and the Audiowide font installed (SIL OFL, Google Fonts; not in the repo)
logo: logo-vector logo-png

logo-vector:
    just _logo exact_units-light.svg lockup light
    just _logo exact_units-dark.svg lockup dark
    just _logo mark-light.svg mark light
    just _logo mark-dark.svg mark dark
    just _logo wordmark-light.svg wordmark light
    just _logo wordmark-dark.svg wordmark dark
    just _logo icon.svg icon dark
    just _logo icon-small.svg icon-small dark

logo-png:
    just _logo png/exact_units-light-640.png lockup light 640
    just _logo png/exact_units-light-1280.png lockup light 1280
    just _logo png/exact_units-dark-640.png lockup dark 640
    just _logo png/exact_units-dark-1280.png lockup dark 1280
    just _logo png/mark-light-256.png mark light 256
    just _logo png/mark-light-512.png mark light 512
    just _logo png/mark-dark-256.png mark dark 256
    just _logo png/mark-dark-512.png mark dark 512
    just _logo png/icon-small-16.png icon-small dark 16
    just _logo png/icon-small-32.png icon-small dark 32
    just _logo png/icon-small-48.png icon-small dark 48
    just _logo png/icon-128.png icon dark 128
    just _logo png/icon-256.png icon dark 256
    just _logo png/icon-512.png icon dark 512
    just _logo png/social.png social dark

# One variant: the output's extension picks the format, size is the longer side in pixels (empty: natural size)
_logo out kind theme size="":
    typst compile --ppi 72 --input kind={{kind}} --input theme={{theme}} --input size={{size}} assets/logo/render.typ assets/logo/{{out}}
