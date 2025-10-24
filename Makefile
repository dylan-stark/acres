build: reformat test

docs:
	cargo doc --workspace --no-deps

docs-all:
	cargo doc --workspace --no-deps --document-private-items

reformat:
	cargo fmt --all --check

lint:
	cargo clippy --all-targets --all-features --workspace -- -D warnings

test:
	cargo test --locked --all-features --workspace

watch:
	cargo watch -x "fmt --all --check" -x "clippy -- -D warnings" -x check -x test
.PHONY: watch

watch-doc:
	cargo watch -x doc

