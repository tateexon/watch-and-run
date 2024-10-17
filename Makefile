.PHONY: lint
lint:
	cargo clippy -- -D warnings
	cargo fmt

.PHONY: build
build: lint
	cargo build

.PHONY: build-release
build-release:
	cargo build --release

.PHONY: run
run: build
	cargo run

.PHONY: test
test:
	cargo test
