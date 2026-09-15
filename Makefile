.PHONY: build test clippy fmt clean

build:
	cargo build --release

test:
	cargo test

clippy:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

clean:
	cargo clean
