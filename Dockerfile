# Use official Rust image as builder
FROM rustlang/rust:nightly as builder
WORKDIR /usr/src/audio-ai

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN apt-get update && apt-get install -y \
    pkg-config libasound2-dev clang libclang-dev \
    libaubio-dev aubio-tools libaubio-doc \
    build-essential \
 && rm -rf /var/lib/apt/lists/*

# rustfmt (optional)
RUN rustup component add rustfmt || true

# --- Scope the fix to aubio only (no global CFLAGS!) ---
# Pre-fetch crates so we can patch vendored aubio sources in the cargo registry
RUN cargo fetch

# 1) Make strncasecmp visible to aubio (patch inside downloaded aubio-sys crate)
RUN find /usr/local/cargo/registry/src -path "*/aubio-sys-*/aubio/src/utils/strutils.c" -print -exec \
    sh -c 'grep -q "<strings.h>" "$1" || sed -i "1i #include <strings.h>" "$1"' _ {} \;
RUN find /usr/local/cargo/registry/src -path "*/aubio-sys-*/aubio/src/utils/strutils.c" -print -exec \
    sh -c 'grep -q "_POSIX_C_SOURCE" "$1" || sed -i "1i #define _POSIX_C_SOURCE 200809L" "$1"' _ {} \;

# 2) (optional) quiet the calloc warning by fixing arg order
RUN find /usr/local/cargo/registry/src -path "*/aubio-sys-*/aubio/src/aubio_priv.h" -print -exec \
    sed -i 's/calloc(sizeof(_t), 1)/calloc(1, sizeof(_t))/' {} \;

RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libaubio-dev aubio-tools libaubio-doc libasound2-dev \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/audio-ai/target/release/audio-ai /app/audio-ai
CMD ["./audio-ai"]