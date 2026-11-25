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

# Records demo.gif inside the dev image (see Dockerfile); output lands in the repo root.
docker-demo:
	docker build -t acres-dev .
	docker run --rm --user "$${SUDO_UID:-$$(id -u)}:$${SUDO_GID:-$$(id -g)}" -e HOME=/tmp -e CARGO_HOME=/tmp/cargo -e CARGO_TARGET_DIR=/work/target-docker -v "$(CURDIR)":/work acres-dev make demo
.PHONY: docker-demo

# Records demo.gif offline: serves the fixture images on 127.0.0.1:8099 and runs vhs against the built TUI.
# Needs vhs, ttyd and Chromium on PATH (all in the Docker image; see `docker-demo`).
demo:
	cargo build --locked -p acres-tui
	@python3 -m http.server 8099 --bind 127.0.0.1 --directory acres-tui/tests/fixtures >/dev/null 2>&1 & \
	server=$$!; trap 'kill $$server' EXIT; \
	PATH="$${CARGO_TARGET_DIR:-$(CURDIR)/target}/debug:$$PATH" ACRES_USE_CACHE=false vhs demo.tape
.PHONY: demo

# Prints a coverage summary (report only, no gate). Needs `cargo install cargo-llvm-cov` and the llvm-tools-preview component.
coverage:
	cargo llvm-cov --locked --all-features --workspace --summary-only
.PHONY: coverage

# Criterion benches: the charaoke stages, the image-to-ascii baseline (see ADR 0008), acres parsing
# (ADR 0016) and whole TUI frames (ADR 0013).
# Every run is compared with the previous one. To compare against a fixed point instead, save a
# named baseline once (`make bench-save`, default name `main`) and compare later runs with
# `make bench-compare`, which leaves the saved baseline untouched. `FILTER` narrows any of them to
# matching benchmark names, e.g. `make bench-compare FILTER=charaoke/`.
BENCH = cargo bench --locked --bench '*' -p charaoke -p image-to-ascii-builder -p acres -p acres-tui
BASELINE ?= main
FILTER ?=

bench:
	$(BENCH) -- $(FILTER)
.PHONY: bench

bench-save:
	$(BENCH) -- --save-baseline $(BASELINE) $(FILTER)
.PHONY: bench-save

bench-compare:
	$(BENCH) -- --baseline $(BASELINE) $(FILTER)
.PHONY: bench-compare

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

