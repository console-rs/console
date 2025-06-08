all: test

check:
	@cargo check --lib --all-features

build:
	@cargo build --all-features

doc:
	@cargo doc --all-features

test:
	@echo "CARGO TESTS"
	@cargo test
	@cargo test --all-features
	@cargo test --lib --no-default-features
	@cargo test --lib --no-default-features --features alloc
	@cargo test --no-default-features --features std
	@cargo test --no-default-features --features std,ansi-parsing
	@cargo test --no-default-features --features std,unicode-width

check-minver:
	@echo "MINVER CHECK"
	@cargo minimal-versions check --lib
	@cargo minimal-versions check --lib --all-features
	@cargo minimal-versions check --lib --no-default-features

format:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all

format-check:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all -- --check

lint:
	@rustup component add clippy 2> /dev/null
	@cargo clippy --examples --tests --all-features -- --deny warnings

msrv-lock:
	@cargo update -p once_cell --precise 1.20.3

.PHONY: all doc build check test format format-check lint check-minver msrv-lock
