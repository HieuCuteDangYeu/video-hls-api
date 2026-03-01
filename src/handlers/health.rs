use actix_web::{get, HttpResponse};

use crate::models::HealthResponse;
use crate::services::ffmpeg;

/// Health check endpoint.
///
/// Returns service status, version, and whether FFmpeg is available.
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
#[get("/api/v1/health")]
pub async fn health_check() -> HttpResponse {
    let resp = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        ffmpeg_available: ffmpeg::is_ffmpeg_available(),
    };
    HttpResponse::Ok().json(resp)
}
