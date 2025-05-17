test_integration:
	cargo test --locked --all-features --workspace -- --ignored

docs:
	cargo doc --no-deps --document-private-items --all-features --workspace --examples

watch:
	cargo watch -x "fmt --check" -x "clippy -- -D warnings" -x check -x test

.PHONY: build reformat lint test docs watch
