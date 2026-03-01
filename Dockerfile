# ── Stage 0: Chef planner (dependency cache) ─────────
FROM rust:1.85-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 1: Build ────────────────────────────────────
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build only dependencies (cached unless Cargo.toml/lock change)
RUN cargo chef cook --release --recipe-path recipe.json
# Build the actual application
COPY . .
RUN cargo build --release

# ── Stage 2: Runtime ─────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r appuser && useradd -r -g appuser -m appuser

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/video-hls-api /app/video-hls-api

# Create work directories
RUN mkdir -p /app/hls_work /app/playlists && chown -R appuser:appuser /app

USER appuser

# Default environment
ENV APP_HOST=0.0.0.0
ENV APP_PORT=8080
ENV WORK_DIR=/app/hls_work
ENV PLAYLISTS_DIR=/app/playlists
ENV RUST_LOG=info

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

CMD ["./video-hls-api"]
