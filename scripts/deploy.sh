#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────
#  deploy.sh — Pull & restart the Docker container on EC2
#  Called by GitHub Actions via SSH
# ──────────────────────────────────────────────────────────
set -euo pipefail

APP_NAME="video-hls-api"
IMAGE_NAME="${DOCKER_IMAGE:-video-hls-api:latest}"
ENV_FILE="/home/ubuntu/video-hls-api/.env"
CONTAINER_PORT=8080
HOST_PORT=8080

echo "🚀 Deploying ${APP_NAME}…"

# Stop & remove the existing container (if any)
docker stop "${APP_NAME}" 2>/dev/null || true
docker rm "${APP_NAME}" 2>/dev/null || true

# Pull the latest image (if using a registry)
if [[ "${IMAGE_NAME}" == *"/"* ]]; then
    echo "📦 Pulling ${IMAGE_NAME}…"
    docker pull "${IMAGE_NAME}"
fi

# Run the new container
echo "▶️  Starting container…"
docker run -d \
    --name "${APP_NAME}" \
    --restart unless-stopped \
    --env-file "${ENV_FILE}" \
    -p "${HOST_PORT}:${CONTAINER_PORT}" \
    -v /home/ubuntu/video-hls-api/playlists:/app/playlists \
    "${IMAGE_NAME}"

# Wait for health check
echo "⏳ Waiting for health check…"
for i in $(seq 1 30); do
    if curl -sf http://localhost:${HOST_PORT}/api/v1/health > /dev/null 2>&1; then
        echo "✅ ${APP_NAME} is healthy!"
        exit 0
    fi
    sleep 2
done

echo "❌ Health check failed after 60s"
docker logs "${APP_NAME}" --tail 50
exit 1
