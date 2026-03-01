# Video HLS API 🎬

A Rust REST API that uploads videos, slices them into HLS segments via FFmpeg, masks each segment with a 1×1 PNG prefix, uploads them to a CDN, and generates M3U8 playlists with `#EXT-X-BYTERANGE` tags for seamless playback.

## Architecture

```
┌────────────┐      ┌──────────────────────────────────────────┐
│  Client     │─────▶│  POST /api/v1/video/upload               │
│  (cURL /    │      │  (multipart/form-data with video file)   │
│   Swagger)  │      └──────────┬───────────────────────────────┘
└────────────┘               │
                             ▼
                   ┌──────────────────┐
                   │  FFmpeg Slicing   │  Strip metadata, produce .ts segments
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │  PNG Masking &    │  Prepend 67-byte 1×1 PNG to each .ts
                   │  CDN Upload       │  Upload as .png → get CDN URL
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │  M3U8 Rewrite     │  Replace local paths with CDN URLs
                   │  + BYTERANGE      │  Add #EXT-X-BYTERANGE:size@67
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │  Response:        │  JSON with playlist + player URL
                   │  UploadResponse   │
                   └──────────────────┘
```

## Prerequisites

- **Rust** ≥ 1.75 (2021 edition)
- **FFmpeg** installed and in `PATH`
- CDN credentials configured in `.env`

## Quick Start

```bash
# 1. Clone and enter the project
cd video-hls-api

# 2. Copy and configure environment
cp .env.example .env
# Edit .env with your CDN credentials

# 3. Build and run
cargo run

# Server starts at http://localhost:8080
# Swagger UI at http://localhost:8080/swagger-ui/
```

## API Endpoints

| Method   | Path                              | Description                        |
|----------|-----------------------------------|------------------------------------|
| `GET`    | `/api/v1/health`                  | Health check + FFmpeg status       |
| `POST`   | `/api/v1/video/upload`            | Upload video → HLS + CDN pipeline |
| `GET`    | `/api/v1/video/{job_id}/playlist` | Get M3U8 playlist for a job        |
| `GET`    | `/api/v1/video/{job_id}/player`   | Get HTML player page for a job     |
| `GET`    | `/api/v1/video/jobs`              | List all processed jobs            |
| `DELETE` | `/api/v1/video/{job_id}`          | Delete a job from memory           |

## Usage Examples

### Upload a video

```bash
curl -X POST http://localhost:8080/api/v1/video/upload \
  -F "video=@my_video.mp4" \
  -F "segment_duration=5"
```

### Get the playlist

```bash
curl http://localhost:8080/api/v1/video/{job_id}/playlist
```

### Open the player

Navigate to `http://localhost:8080/api/v1/video/{job_id}/player` in your browser.

## Project Structure

```
video-hls-api/
├── Cargo.toml
├── .env.example
├── .env
├── .gitignore
├── README.md
└── src/
    ├── main.rs              # Server bootstrap, Swagger, routing
    ├── config.rs            # Environment-based configuration
    ├── errors.rs            # AppError + ErrorResponse (Swagger-aware)
    ├── models.rs            # Request/Response/Internal data types
    ├── handlers/
    │   ├── mod.rs
    │   ├── health.rs        # GET /health
    │   └── video.rs         # POST /upload, GET /playlist, GET /player, etc.
    └── services/
        ├── mod.rs
        ├── ffmpeg.rs        # FFmpeg slicing logic
        ├── upload.rs        # PNG masking + CDN upload
        └── hls.rs           # M3U8 rewriting + HTML player generation
```

## Environment Variables

| Variable               | Required | Default    | Description                          |
|------------------------|----------|------------|--------------------------------------|
| `APP_HOST`             | No       | `0.0.0.0`  | Server bind host                     |
| `APP_PORT`             | No       | `8080`     | Server bind port                     |
| `WORK_DIR`             | No       | `./hls_work` | Temp directory for processing      |
| `CDN_UPLOAD_ENDPOINT`  | **Yes**  | —          | Full CDN upload URL                  |
| `CDN_CSRF_TOKEN`       | **Yes**  | —          | CSRF token for CDN auth              |
| `CDN_UUID`             | **Yes**  | —          | UUID header for CDN auth             |
| `CDN_COOKIE`           | **Yes**  | —          | Cookie header for CDN auth           |
| `HLS_SEGMENT_DURATION` | No       | `5`        | Default segment duration (seconds)   |
| `MAX_UPLOAD_SIZE_MB`   | No       | `500`      | Max upload file size (MB)            |
| `RUST_LOG`             | No       | `info`     | Log level (`debug`, `info`, `warn`)  |

## How It Works

1. **Upload** — Client sends a video file via multipart form
2. **Slice** — FFmpeg splits it into `.ts` segments, stripping all metadata
3. **Mask** — Each `.ts` segment is prepended with a 67-byte valid 1×1 PNG header
4. **Upload** — Masked files are uploaded to the CDN as `.png` images
5. **Rewrite** — The M3U8 playlist replaces local filenames with CDN URLs and adds `#EXT-X-BYTERANGE:size@67` to skip the PNG header during playback
6. **Serve** — The API returns the playlist and a self-contained HTML player

## License

MIT
