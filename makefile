run-local:
	cargo run --release -- --dev

run:
	cargo run --release -- --dev --tmp

build:
	cargo build --release

test:
	cargo test

check:
	cargo check --all --tests

lint:
	cargo clippy --all-targets

purge:
	cargo run --release -- purge-chain --dev