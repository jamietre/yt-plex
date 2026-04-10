# ── Stage 1: Build the SvelteKit frontend ────────────────────────────────────
FROM node:22-bookworm-slim AS web-builder
WORKDIR /app

RUN npm install -g pnpm@10

COPY web/package.json web/pnpm-lock.yaml ./web/
RUN cd web && pnpm install --frozen-lockfile

COPY web/ ./web/
RUN cd web && pnpm build


# ── Stage 2: Build the Rust binary ───────────────────────────────────────────
FROM rust:1.88-bookworm AS rust-builder
WORKDIR /app

# Copy workspace manifests first for layer-cached dependency builds.
COPY Cargo.toml Cargo.lock ./
COPY crates/common/Cargo.toml ./crates/common/
COPY crates/server/Cargo.toml  ./crates/server/

# Stub sources so cargo can resolve the dependency graph and cache deps.
RUN mkdir -p crates/common/src crates/server/src && \
    echo 'pub fn main() {}' > crates/common/src/lib.rs && \
    echo 'fn main() {}' > crates/server/src/main.rs && \
    mkdir -p web/build && touch web/build/.keep && \
    cargo build --release -p yt-plex-server 2>/dev/null || true

# Copy real sources (including the web build output rust-embed needs).
COPY crates/ ./crates/
COPY --from=web-builder /app/web/build/ ./web/build/

# Touch sources to invalidate the stub build, then build for real.
RUN touch crates/common/src/lib.rs crates/server/src/main.rs && \
    cargo build --release -p yt-plex-server


# ── Stage 3: Runtime image ───────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        ffmpeg \
        curl \
        python3 \
    && rm -rf /var/lib/apt/lists/*

# Install yt-dlp from the official GitHub release binary.
RUN curl -fsSL https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp \
        -o /usr/local/bin/yt-dlp && \
    chmod +x /usr/local/bin/yt-dlp

COPY --from=rust-builder /app/target/release/yt-plex /usr/local/bin/yt-plex

# /config  — mount your config.toml here
# /data    — persistent volume for the SQLite DB and thumbnail cache
# /media   — bind-mount your Plex media output directory here
RUN mkdir -p /config /data /media

ENV YT_PLEX_CONFIG=/config/config.toml

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/yt-plex"]
