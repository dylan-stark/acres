watch:
	cargo watch -x "clippy -- -D warnings" -x check -x test
.PHONY: watch

watch-doc:
	cargo watch -x doc

