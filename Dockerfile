# ==========================================
# Stage 1: Build the Rust application
# ==========================================
FROM rust:bookworm AS builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Pre-fetch and cache dependencies
COPY Cargo.toml Cargo.lock ./

RUN mkdir -p src/bin \
    && echo "pub fn dummy() {}" > src/lib.rs \
    && echo "fn main() {}" > src/main.rs \
    && echo "fn main() {}" > src/bin/add_song.rs \
    && cargo build --release \
    && rm -rf src \
              target/release/.fingerprint/sexual_tiger* \
              target/release/deps/*sexual_tiger* \
              target/release/deps/*add_song* \
              target/release/sexual_tiger* \
              target/release/add_song*

# Copy actual source code and migrations
COPY src ./src
COPY migrations ./migrations

# Build the real release binaries and library
RUN cargo build --release

# ==========================================
# Stage 2: Minimal runtime image
# ==========================================
FROM debian:bookworm-slim AS runtime

WORKDIR /usr/src/app

# Install runtime dependencies:
# - ca-certificates: HTTPS TLS certificates (Discord API, YouTube, XKCD)
# - ffmpeg: audio decoding and transcoding for voice streaming
# - yt-dlp & python3: downloading audio from YouTube and extracting metadata
# - libssl3 & libsqlite3-0: shared system libraries for networking and SQLite
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    ffmpeg \
    libssl3 \
    libsqlite3-0 \
    python3 \
    yt-dlp \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries from builder
COPY --from=builder /usr/src/app/target/release/sexual_tiger /usr/local/bin/sexual_tiger
COPY --from=builder /usr/src/app/target/release/add_song /usr/local/bin/add_song

# Copy entrypoint script
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Create app data directories
RUN mkdir -p /usr/src/app/songs /usr/src/app/audio /usr/src/app/images

# Sane defaults
ENV DATABASE_URL="playlists.sql" \
    SONG_DIRECTORY="songs/" \
    RUST_LOG="info"

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["sexual_tiger"]
