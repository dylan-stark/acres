# Reproducible dev/CI environment: pinned Rust toolchain plus the tools used by `make check` and `demo.tape`.
# Keep the Rust version in sync with rust-toolchain.toml and .github/workflows/*.yml.
FROM golang:1 AS vhs
RUN go install github.com/charmbracelet/vhs@latest

FROM rust:1.99.0
RUN apt-get update \
    && apt-get install -y --no-install-recommends jq ffmpeg make git curl ca-certificates python3 \
    && rm -rf /var/lib/apt/lists/*
# ttyd isn't packaged for this Debian release; use the upstream static binary.
RUN curl -fsSL -o /usr/local/bin/ttyd \
        "https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.$(uname -m)" \
    && chmod +x /usr/local/bin/ttyd
RUN rustup component add clippy rustfmt
COPY --from=vhs /go/bin/vhs /usr/local/bin/vhs
WORKDIR /work
CMD ["make", "check"]
