# Makefile — convenience commands for webui-companion
.PHONY: all fmt clippy check test clean ci setup-hooks lint markdownlint prettier

all: check

fmt:
	cargo fmt --all

clippy:
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

# clippy already runs check internally — this alias is pure fmt + clippy
check: fmt clippy

markdownlint:
	prek run --all-files markdownlint-cli2

prettier:
	prek run --all-files prettier

test:
	cargo test --locked --workspace --all-targets --all-features

clean:
	cargo clean

# CI gate — everything that must pass before merge
ci: check prettier markdownlint test

# Configure git hooks via prek (one-time setup, or after clone)
setup-hooks:
	prek install
	@echo "Git hooks installed via prek"

lint:
	cargo clippy --locked --workspace --all-targets --all-features -- -W clippy::pedantic -W clippy::nursery -D warnings
