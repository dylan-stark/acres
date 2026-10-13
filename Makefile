build: reformat-check test

check: check-toolchain reformat-check lint test docs
.PHONY: check

# Fails if the active rustc differs from the version pinned in rust-toolchain.toml (and CI).
check-toolchain:
	@pinned=$$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml); \
	actual=$$(rustc --version | cut -d' ' -f2); \
	if [ "$$pinned" != "$$actual" ]; then \
		echo "rustc $$actual != pinned $$pinned (rust-toolchain.toml); install rustup so the pin takes effect" >&2; exit 1; \
	fi
.PHONY: check-toolchain

# Builds the dev image and runs `make check` inside it (see Dockerfile).
docker-check:
	docker build -t acres-dev .
	docker run --rm --user "$${SUDO_UID:-$$(id -u)}:$${SUDO_GID:-$$(id -g)}" -e HOME=/tmp -e CARGO_HOME=/tmp/cargo -e CARGO_TARGET_DIR=/work/target-docker -v "$(CURDIR)":/work acres-dev
.PHONY: docker-check

# Records demo.gif offline: serves the fixture images on 127.0.0.1:8099 and runs vhs against the built TUI.
# Needs vhs and ttyd on PATH (both are in the Docker image).
demo:
	cargo build --locked -p acres-tui
	@python3 -m http.server 8099 --bind 127.0.0.1 --directory acres-tui/tests/fixtures >/dev/null 2>&1 & \
	server=$$!; trap 'kill $$server' EXIT; \
	PATH="$(CURDIR)/target/debug:$$PATH" ACRES_USE_CACHE=false vhs demo.tape
.PHONY: demo

# Prints a coverage summary (report only, no gate). Needs `cargo install cargo-llvm-cov` and the llvm-tools-preview component.
coverage:
	cargo llvm-cov --locked --all-features --workspace --summary-only
.PHONY: coverage

docs:
	cargo doc --workspace --no-deps

docs-all:
	cargo doc --workspace --no-deps --document-private-items

reformat:
	cargo fmt --all

reformat-check:
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

