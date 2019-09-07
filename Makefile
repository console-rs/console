all: test
.PHONY: all

format:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all
.PHONY: format

format-check:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all -- --check
.PHONY: format-check

lint:
	@rustup component add clippy 2> /dev/null
	@cargo clippy
.PHONY: lint

update-readme:
	@cargo readme > README.md
.PHONY: update-readme

test:
	@cargo test
	@cargo test --no-default-features
.PHONY: test
