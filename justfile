# List all available targets
default:
    @just --list

# The tests, the feature sets of CI's test job. `--changed` (what a session runs before it is ready) is the same run:
# the repository is one crate. Warnings are errors, as in CI (its toolchain action sets `-D warnings`): a test config
# that warns fails here too. Not `--all-features`: with `defmt` host test binaries need a global logger (ci.yml).
test *flags:
    CARGO_BUILD_WARNINGS=deny cargo nextest run --features shrink_wrap,customary,serde --no-fail-fast
    CARGO_BUILD_WARNINGS=deny cargo nextest run --no-default-features --features std --no-fail-fast
    CARGO_BUILD_WARNINGS=deny cargo nextest run --no-default-features --features shrink_wrap,quantities,serde --no-fail-fast
    # nextest does not run doctests; they include the book's pages (src/book.rs)
    CARGO_BUILD_WARNINGS=deny cargo test --doc --features shrink_wrap,customary,serde
    # the book site's Markdown extensions (docs_ext: the `bits` fence, `rust,compile_fail` blocks)
    uv run --group dev pytest -q docs_ext

# The Kani proofs in tests/props: the properties the proptests draw inputs for, proved for every input. Needs Kani
# (`cargo install --locked kani-verifier && cargo kani setup`); takes minutes, so it isn't part of `just test`. `serde`: the tests/serde.rs target requires it, and Kani refuses a target whose features are off
verify:
    cargo kani --tests --features serde

# Mutation testing: change the library one mutation at a time (.cargo/mutants.toml) and list the mutants no test
# catches, in mutants.out/. Needs cargo-mutants (`cargo install --locked cargo-mutants`); hours, so not in CI. Extra
# flags pass through: `-f src/index.rs` for one file, `-j 8` for parallel jobs
mutants *flags:
    cargo mutants --features shrink_wrap,customary,serde {{flags}}

# Fuzz one target of fuzz/ for SECONDS: the property functions of tests/props, coverage-guided. Needs a nightly
# toolchain and cargo-fuzz (`rustup toolchain install nightly`, `cargo install cargo-fuzz`). Starts from the
# hand-written seeds in fuzz/seeds/TARGET; the corpus it grows (fuzz/corpus/TARGET) and a crash (fuzz/artifacts/TARGET)
# stay out of git. Targets: decimal, decibels, names, wire_bytes, wire_values, serde_json. ASan keeps the stack of
# every allocation for good: 4 frames of it, so the deep recursion of unit trees doesn't grow it past the 2 GB limit
[working-directory('fuzz')]
fuzz target seconds="60":
    mkdir -p corpus/{{target}}
    ASAN_OPTIONS=malloc_context_size=4 cargo +nightly fuzz run {{target}} corpus/{{target}} seeds/{{target}} -- -dict=units.dict -max_total_time={{seconds}} -max_len=4096

# Format, sort, spelling, clippy and rustdoc as CI runs them, warnings are errors
lint:
    cargo fmt --check
    cargo sort --check
    typos
    cargo clippy --all-targets -- -D warnings
    cargo clippy --all-targets --all-features -- -D warnings
    cargo clippy --all-targets --no-default-features --features std -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
    cargo clippy --manifest-path tools/book_plots/Cargo.toml --target-dir target -- -D warnings

# cargo check the feature sets a firmware and a host use: no unit group, defmt, a few groups, without and with the wire format, CORDIC on an M0
check:
    cargo check --all-features
    cargo check --no-default-features
    cargo check --no-default-features --features=defmt
    cargo check --no-default-features --features=time,information,counting,dimensionless
    cargo check --no-default-features --features=std
    cargo check --no-default-features --features=shrink_wrap,defmt,time,serde
    cargo check --no-default-features --features=electrical,libm --target thumbv7em-none-eabihf
    cargo check --no-default-features --features=angle,dimensionless,counting,serde --target thumbv6m-none-eabi

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

# Serve the book (docs/, zensical.toml) at http://localhost:8000 and rebuild it on every change
docs: docs-prep
    uv run zensical serve

# Build the book into site/, strict as CI builds it: a warning fails the build
docs-build: docs-prep
    uv run zensical build --clean --strict

# What the book needs beside its pages, none of it in git: KaTeX (pinned, hash-checked), the logo, the plots the
# crate computes (tools/book_plots)
docs-prep:
    uv run python -m eu_docs_ext.vendor docs
    mkdir -p docs/assets/logo
    cp assets/logo/*.svg docs/assets/logo/
    cargo run -q --manifest-path tools/book_plots/Cargo.toml --target-dir target -- docs/plots

# Regenerate the tables of docs/customary.md from the sourced catalogue of customary units
customary-docs:
    CUSTOMARY_DOCS=update cargo nextest run --all-features docs_page_tables

# Collect coverage for the feature sets `just test` uses (shared by cov and cov-html)
_cov-collect:
    cargo llvm-cov clean --workspace
    cargo llvm-cov nextest --no-report --features shrink_wrap,customary,serde
    cargo llvm-cov nextest --no-report --no-default-features --features std
    cargo llvm-cov nextest --no-report --no-default-features --features shrink_wrap,quantities,serde

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
