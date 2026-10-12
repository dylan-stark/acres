# Reproducible dev/CI environment: pinned Rust toolchain plus the tools used by `make check` and `demo.tape`.
# Keep the Rust version in sync with rust-toolchain.toml and .github/workflows/*.yml.
FROM golang:1 AS vhs
RUN go install github.com/charmbracelet/vhs@latest

FROM rust:1.99.0
RUN apt-get update \
    && apt-get install -y --no-install-recommends jq ffmpeg ttyd make git \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add clippy rustfmt
COPY --from=vhs /go/bin/vhs /usr/local/bin/vhs
WORKDIR /work
CMD ["make", "check"]
