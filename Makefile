.PHONY: fmt, lint, run-release, run-debug
.DEFAULT: all

all: fmt lint run-debug

fmt:
	cargo fmt

lint:
	cargo clippy

run-release:
	cargo run --release

run-debug:
	cargo run