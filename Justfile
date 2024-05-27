# just manual: https://github.com/casey/just#readme

_default:
	just --list

# Run all tests with nextest and cargo-llvm-cov
ci-test:
    #!/bin/bash -eux
    just build-testbed
    just cov

cov:
    #!/bin/bash -eux
    just build-testbed
    export RUSTUP_TOOLCHAIN=nightly-2024-05-26
    rm -rf coverage
    mkdir -p coverage
    cargo llvm-cov nextest --branch --ignore-filename-regex '.*crates/(httpwg|fluke-hyper-testbed|fluke-tls-sample|fluke-sample-h2-server).*' --html --output-dir=coverage
    cargo llvm-cov report --lcov --output-path 'coverage/lcov.info'

# Run all tests with cargo nextest
test *args:
	just build-testbed
	export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
	cargo nextest run {{args}}

build-testbed:
	cargo build --release -p fluke-hyper-testbed

single-test *args:
	just test --no-capture {{args}}

check:
	#!/bin/bash -eu
	cargo clippy --all-targets --all-features

tls-sample:
	cargo run -p fluke-tls-sample

httpwg-gen:
    cargo run --release --package httpwg-gen
