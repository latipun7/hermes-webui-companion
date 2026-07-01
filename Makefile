# Makefile — convenience commands for webui-companion
.PHONY: all fmt clippy check test clean ci setup-hooks lint

all: check

fmt:
	cargo fmt --all

clippy:
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

# clippy already runs check internally — this alias is pure fmt + clippy
check: fmt clippy

test:
	cargo test --locked --workspace --all-targets --all-features

clean:
	cargo clean

# CI gate — everything that must pass before merge
ci: check test

# Configure git to use .githooks (one-time setup, or after clone)
setup-hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks path set to .githooks/"

lint:
	cargo clippy --locked --workspace --all-targets --all-features -- -W clippy::pedantic -W clippy::nursery -D warnings
