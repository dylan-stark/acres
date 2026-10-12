build: reformat test

check: check-toolchain reformat lint test docs
.PHONY: check

# Fails if the active rustc differs from the version pinned in rust-toolchain.toml (and CI).
check-toolchain:
	@pinned=$$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml); \
	actual=$$(rustc --version | cut -d' ' -f2); \
	if [ "$$pinned" != "$$actual" ]; then \
		echo "rustc $$actual != pinned $$pinned (rust-toolchain.toml); install rustup so the pin takes effect" >&2; exit 1; \
	fi
.PHONY: check-toolchain

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

