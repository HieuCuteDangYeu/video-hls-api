use std::path::Path;
use tracing::info;

use crate::errors::AppError;
use crate::models::SegmentUploadResult;
use crate::services::upload::PNG_MASK_SIZE;

/// Rewrite the local M3U8 playlist to point at remote CDN URLs
/// with `#EXT-X-BYTERANGE` tags that skip the 67-byte PNG mask.
///
/// Also upgrades the HLS version from 3 → 4 (required for BYTERANGE support).
///
/// # Arguments
/// * `playlist_path` – Path to the original `index.m3u8` on disk
/// * `upload_results` – Results from the CDN upload step
///
/// # Returns
/// The final M3U8 playlist as a `String`.
pub fn rewrite_playlist(
    playlist_path: &Path,
    upload_results: &[SegmentUploadResult],
) -> Result<String, AppError> {
    let mut m3u8 = std::fs::read_to_string(playlist_path)?;

    for result in upload_results {
        // Replace the local filename with a BYTERANGE tag + remote URL.
        // BYTERANGE: <size>@<offset>  — size = original TS bytes, offset = 67 (skip PNG).
        let byterange_block = format!(
            "#EXT-X-BYTERANGE:{}@{}\n{}",
            result.original_size, PNG_MASK_SIZE, result.remote_url
        );
        m3u8 = m3u8.replace(&result.filename, &byterange_block);
    }

    // Upgrade HLS version for BYTERANGE support
    m3u8 = m3u8.replace("#EXT-X-VERSION:3", "#EXT-X-VERSION:4");

    info!("Playlist rewritten with {} BYTERANGE entries", upload_results.len());
    Ok(m3u8)
}

/// Save the final M3U8 playlist to a local file.
///
/// # Arguments
/// * `output_dir` – Directory where the M3U8 file will be saved
/// * `job_id` – Job identifier used in the filename
/// * `m3u8_content` – The rewritten playlist content
///
/// # Returns
/// The absolute path to the saved file.
pub fn save_playlist(
    output_dir: &std::path::Path,
    job_id: &str,
    m3u8_content: &str,
) -> Result<std::path::PathBuf, crate::errors::AppError> {
    std::fs::create_dir_all(output_dir)?;
    let file_path = output_dir.join(format!("{}.m3u8", job_id));
    std::fs::write(&file_path, m3u8_content)?;
    info!("Saved playlist to {:?}", file_path);
    Ok(file_path)
}
