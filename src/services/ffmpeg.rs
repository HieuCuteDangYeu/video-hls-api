use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, error};

use crate::errors::AppError;
use crate::models::HlsSegment;

/// Check if FFmpeg is installed and accessible.
pub fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Slice a video file into HLS segments using FFmpeg.
///
/// This strips all metadata (ID3 tags) so the TS segments appear as raw data,
/// then produces `.ts` segments and an `index.m3u8` playlist.
///
/// # Arguments
/// * `input_path` – Path to the uploaded video file
/// * `output_dir` – Directory to write segments + playlist into
/// * `segment_duration` – Duration of each HLS segment in seconds
///
/// # Returns
/// A vector of `HlsSegment` descriptors for every `.ts` file produced.
pub fn slice_video(
    input_path: &Path,
    output_dir: &Path,
    segment_duration: u32,
) -> Result<Vec<HlsSegment>, AppError> {
    // Ensure the output directory exists
    std::fs::create_dir_all(output_dir)?;

    let m3u8_path = output_dir.join("index.m3u8");

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            input_path.to_str().unwrap_or_default(),
            "-codec:",
            "copy",
            // Strip all metadata — makes the TS segments look like raw data
            "-map_metadata",
            "-1",
            "-metadata",
            "service_provider=video-hls-api",
            "-metadata",
            "service_name=video-hls-api",
            "-start_number",
            "0",
            "-hls_time",
            &segment_duration.to_string(),
            "-hls_list_size",
            "0",
            "-f",
            "hls",
            m3u8_path.to_str().unwrap_or_default(),
        ])
        .output();

    match status {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("FFmpeg failed: {}", stderr);
                return Err(AppError::FfmpegError(stderr.to_string()));
            }
            info!("FFmpeg slicing completed successfully");
        }
        Err(e) => {
            error!("Failed to execute FFmpeg: {}", e);
            return Err(AppError::FfmpegError(format!(
                "Failed to execute FFmpeg: {}. Is FFmpeg installed?",
                e
            )));
        }
    }

    // Collect all .ts segment files
    let mut segments = collect_segments(output_dir)?;
    segments.sort_by(|a, b| a.filename.cmp(&b.filename));

    info!("Produced {} HLS segments", segments.len());
    Ok(segments)
}

/// Scan a directory for `.ts` files and return segment descriptors.
fn collect_segments(dir: &Path) -> Result<Vec<HlsSegment>, AppError> {
    let mut segments = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("ts") {
            let metadata = std::fs::metadata(&path)?;
            segments.push(HlsSegment {
                filename: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: path.clone(),
                size: metadata.len(),
            });
        }
    }

    Ok(segments)
}

/// Get the path to the generated M3U8 playlist inside the output dir.
pub fn playlist_path(output_dir: &Path) -> PathBuf {
    output_dir.join("index.m3u8")
}
