use actix_multipart::Multipart;
use actix_web::{post, web, HttpResponse};
use futures_util::StreamExt;
use tracing::{error, info};

use crate::errors::{AppError, ErrorResponse};
use crate::handlers::video::AppState;
use crate::models::*;
use crate::services::upload;

/// Maximum number of images allowed per request.
const MAX_IMAGES_PER_REQUEST: usize = 20;

/// Upload one or more image files directly to the CDN.
///
/// Send images as a `multipart/form-data` request with one or more fields named `images`.
/// Each file must be a valid image (JPEG, PNG, GIF, WebP, BMP).
/// Maximum total payload: 50 MB (configurable via `MAX_IMAGE_UPLOAD_SIZE_MB`).
#[utoipa::path(
    post,
    path = "/api/v1/image/upload",
    tag = "Image",
    request_body(
        content = ImageUploadRequest,
        content_type = "multipart/form-data",
        description = "Upload one or more image files. Field name must be `images`. Supports JPEG, PNG, GIF, WebP, BMP.",
    ),
    responses(
        (status = 200, description = "Images uploaded successfully", body = ImageUploadResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 502, description = "CDN upload failed", body = ErrorResponse),
        (status = 500, description = "Internal processing error", body = ErrorResponse),
    )
)]
#[post("/api/v1/image/upload")]
pub async fn upload_images(
    state: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let mut files: Vec<(String, Vec<u8>, String)> = Vec::new(); // (filename, data, content_type)
    let max_total_bytes = state.config.max_image_upload_size_mb as usize * 1024 * 1024;
    let mut total_bytes: usize = 0;

    // ── Parse multipart fields ──────────────────────────────────
    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| AppError::MultipartError(e.to_string()))?;

        let field_name = field
            .content_disposition()
            .and_then(|cd| cd.get_name().map(|s| s.to_string()))
            .unwrap_or_default();

        if field_name != "images" {
            // Drain unknown fields
            while field.next().await.is_some() {}
            continue;
        }

        if files.len() >= MAX_IMAGES_PER_REQUEST {
            return Err(AppError::ValidationError(format!(
                "Too many files. Maximum {} images per request",
                MAX_IMAGES_PER_REQUEST
            )));
        }

        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("image_{}.png", files.len()));

        let content_type = field
            .content_type()
            .map(|ct| ct.to_string())
            .unwrap_or_else(|| "image/png".to_string());

        // Validate content type
        let allowed_types = [
            "image/jpeg",
            "image/png",
            "image/gif",
            "image/webp",
            "image/bmp",
        ];
        if !allowed_types.iter().any(|t| content_type.starts_with(t)) {
            return Err(AppError::ValidationError(format!(
                "Unsupported image type '{}' for file '{}'. Allowed: JPEG, PNG, GIF, WebP, BMP",
                content_type, filename
            )));
        }

        let mut buf = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|e| AppError::MultipartError(e.to_string()))?;
            buf.extend_from_slice(&chunk);
        }

        if buf.is_empty() {
            return Err(AppError::ValidationError(format!(
                "File '{}' is empty",
                filename
            )));
        }

        total_bytes += buf.len();
        if total_bytes > max_total_bytes {
            return Err(AppError::ValidationError(format!(
                "Total upload size exceeds {} MB limit",
                state.config.max_image_upload_size_mb
            )));
        }

        info!(
            "Received image: {} ({} bytes, {})",
            filename,
            buf.len(),
            content_type
        );
        files.push((filename, buf, content_type));
    }

    if files.is_empty() {
        return Err(AppError::ValidationError(
            "No images provided. Use field name `images`".to_string(),
        ));
    }

    info!("📸 Uploading {} image(s) to CDN…", files.len());

    // ── Upload each image to CDN ────────────────────────────────
    let mut results: Vec<ImageResult> = Vec::with_capacity(files.len());
    let mut success_count = 0usize;
    let mut failed_count = 0usize;

    for (filename, data, content_type) in &files {
        match upload::upload_image(
            &state.http_client,
            &state.config,
            filename,
            data,
            content_type,
        )
        .await
        {
            Ok(url) => {
                info!("✅ Uploaded {} → {}", filename, &url[..url.len().min(80)]);
                results.push(ImageResult {
                    filename: filename.clone(),
                    url: Some(url),
                    error: None,
                });
                success_count += 1;
            }
            Err(e) => {
                error!("❌ Failed to upload {}: {}", filename, e);
                results.push(ImageResult {
                    filename: filename.clone(),
                    url: None,
                    error: Some(e.to_string()),
                });
                failed_count += 1;
            }
        }
    }

    info!(
        "📸 Image upload complete: {} succeeded, {} failed",
        success_count, failed_count
    );

    let response = ImageUploadResponse {
        total: files.len(),
        uploaded: success_count,
        failed: failed_count,
        results,
    };

    Ok(HttpResponse::Ok().json(response))
}
