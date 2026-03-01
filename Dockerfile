# ── Stage 1: Build ────────────────────────────────────
FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Copy everything and build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release -j 2

# ── Stage 2: Runtime ─────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    ca-certificates \
    libssl3 \
    curl \
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
