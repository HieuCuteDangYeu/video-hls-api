use reqwest::Client;
use tracing::{info, warn, error};

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{CdnUploadResponse, HlsSegment, SegmentUploadResult};

/// The complete, mathematically perfect 1×1 RGBA PNG (67 bytes).
/// Used as a prefix mask so the CDN sees a valid PNG header.
const PNG_HEX: &str = "89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000a49444154789c63000100000500010d0a2db40000000049454e44ae426082";

/// Size of the PNG mask in bytes.
pub const PNG_MASK_SIZE: usize = 67;

/// Build the 67-byte PNG mask buffer (cached on first call via `once_cell` pattern).
fn png_mask() -> Vec<u8> {
    hex::decode(PNG_HEX).expect("PNG_HEX is a valid hex string")
}

/// Upload a single HLS segment to the CDN.
///
/// The segment data is prepended with the 67-byte 1×1 PNG so the CDN
/// accepts it as an image. The filename extension is changed to `.png`.
///
/// # Arguments
/// * `client` – Shared reqwest HTTP client
/// * `config` – Application configuration (holds CDN endpoint + auth)
/// * `segment` – The HLS segment to upload
///
/// # Returns
/// The remote CDN URL on success.
pub async fn upload_segment(
    client: &Client,
    config: &AppConfig,
    segment: &HlsSegment,
) -> Result<SegmentUploadResult, AppError> {
    let ts_data = std::fs::read(&segment.path)?;
    let original_size = ts_data.len() as u64;

    // Prepend the PNG mask to the TS data
    let mask = png_mask();
    let mut spoofed = Vec::with_capacity(mask.len() + ts_data.len());
    spoofed.extend_from_slice(&mask);
    spoofed.extend_from_slice(&ts_data);

    // Rename .ts → .png for the upload
    let upload_filename = segment.filename.replace(".ts", ".png");

    let file_part = reqwest::multipart::Part::bytes(spoofed)
        .file_name(upload_filename.clone())
        .mime_str("image/png")
        .map_err(|e| AppError::UploadError(e.to_string()))?;

    let form = reqwest::multipart::Form::new().part("Filedata", file_part);

    info!("Uploading segment {} ({} bytes + {} mask)", segment.filename, original_size, PNG_MASK_SIZE);

    let response = client
        .post(&config.cdn_upload_endpoint)
        .header("x-ttam-uuid", &config.cdn_uuid)
        .header("x-csrftoken", &config.cdn_csrf_token)
        .header("Cookie", &config.cdn_cookie)
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        error!("CDN responded with HTTP {}: {}", status, body);
        return Err(AppError::UploadError(format!(
            "CDN HTTP {}: {}",
            status, body
        )));
    }

    let cdn_response: CdnUploadResponse = serde_json::from_str(&body).map_err(|e| {
        AppError::UploadError(format!("Failed to parse CDN response: {} — body: {}", e, body))
    })?;

    match (cdn_response.code, cdn_response.data) {
        (Some(0), Some(data)) if data.url.is_some() => {
            let remote_url = data.url.unwrap();
            info!("✅ Uploaded {} → {}", segment.filename, &remote_url[..remote_url.len().min(80)]);
            Ok(SegmentUploadResult {
                filename: segment.filename.clone(),
                remote_url,
                original_size,
            })
        }
        _ => {
            warn!("CDN upload returned non-success for {}: {}", segment.filename, body);
            Err(AppError::UploadError(format!(
                "CDN rejected {}: {}",
                segment.filename, body
            )))
        }
    }
}

/// Upload a single image file directly to the CDN (no masking/prefix).
///
/// # Arguments
/// * `client` – Shared reqwest HTTP client
/// * `config` – Application configuration (holds CDN endpoint + auth)
/// * `filename` – Original filename
/// * `data` – Raw image bytes
/// * `content_type` – MIME type of the image
///
/// # Returns
/// The remote CDN URL on success.
pub async fn upload_image(
    client: &Client,
    config: &AppConfig,
    filename: &str,
    data: &[u8],
    content_type: &str,
) -> Result<String, AppError> {
    let file_part = reqwest::multipart::Part::bytes(data.to_vec())
        .file_name(filename.to_string())
        .mime_str(content_type)
        .map_err(|e| AppError::UploadError(e.to_string()))?;

    let form = reqwest::multipart::Form::new().part("Filedata", file_part);

    info!("Uploading image {} ({} bytes)", filename, data.len());

    let response = client
        .post(&config.cdn_upload_endpoint)
        .header("x-ttam-uuid", &config.cdn_uuid)
        .header("x-csrftoken", &config.cdn_csrf_token)
        .header("Cookie", &config.cdn_cookie)
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        error!("CDN responded with HTTP {}: {}", status, body);
        return Err(AppError::UploadError(format!(
            "CDN HTTP {}: {}",
            status, body
        )));
    }

    let cdn_response: CdnUploadResponse = serde_json::from_str(&body).map_err(|e| {
        AppError::UploadError(format!("Failed to parse CDN response: {} — body: {}", e, body))
    })?;

    match (cdn_response.code, cdn_response.data) {
        (Some(0), Some(data)) if data.url.is_some() => Ok(data.url.unwrap()),
        _ => Err(AppError::UploadError(format!(
            "CDN rejected {}: {}",
            filename, body
        ))),
    }
}

/// Upload all segments sequentially and collect results.
///
/// Segments that fail to upload are logged but do not abort the whole batch.
pub async fn upload_all_segments(
    client: &Client,
    config: &AppConfig,
    segments: &[HlsSegment],
) -> Vec<SegmentUploadResult> {
    let mut results = Vec::with_capacity(segments.len());

    for segment in segments {
        match upload_segment(client, config, segment).await {
            Ok(result) => results.push(result),
            Err(e) => {
                error!("Failed to upload {}: {}", segment.filename, e);
            }
        }
    }

    results
}
