use actix_web::{HttpResponse, ResponseError};
use std::fmt;

/// Application-wide error types.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("FFmpeg processing failed: {0}")]
    FfmpegError(String),

    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Upload to CDN failed: {0}")]
    UploadError(String),

    #[error("HTTP client error: {0}")]
    HttpClientError(#[from] reqwest::Error),

    #[error("Invalid input: {0}")]
    ValidationError(String),

    #[error("Job not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Multipart error: {0}")]
    MultipartError(String),
}

/// Swagger-compatible error response body.
#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    /// HTTP status code
    pub status: u16,
    /// Human-readable error message
    pub message: String,
    /// Error category
    pub error: String,
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.status, self.error, self.message)
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status, error_type) = match self {
            AppError::FfmpegError(_) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "FFMPEG_ERROR",
            ),
            AppError::IoError(_) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "IO_ERROR",
            ),
            AppError::UploadError(_) => (
                actix_web::http::StatusCode::BAD_GATEWAY,
                "UPLOAD_ERROR",
            ),
            AppError::HttpClientError(_) => (
                actix_web::http::StatusCode::BAD_GATEWAY,
                "HTTP_CLIENT_ERROR",
            ),
            AppError::ValidationError(_) => (
                actix_web::http::StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
            ),
            AppError::NotFound(_) => (
                actix_web::http::StatusCode::NOT_FOUND,
                "NOT_FOUND",
            ),
            AppError::InternalError(_) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
            ),
            AppError::MultipartError(_) => (
                actix_web::http::StatusCode::BAD_REQUEST,
                "MULTIPART_ERROR",
            ),
        };

        HttpResponse::build(status).json(ErrorResponse {
            status: status.as_u16(),
            message: self.to_string(),
            error: error_type.to_string(),
        })
    }
}
