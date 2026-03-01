use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ──────────────────────────────────────────────
//  Request / Response models
// ──────────────────────────────────────────────

/// Response returned when a video upload + HLS processing completes.
#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    /// Unique job identifier
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub job_id: String,
    /// Processing status
    #[schema(example = "completed")]
    pub status: JobStatus,
    /// Number of TS segments produced
    #[schema(example = 12)]
    pub segments_count: usize,
    /// Number of segments successfully uploaded to CDN
    #[schema(example = 12)]
    pub segments_uploaded: usize,
    /// The final M3U8 playlist content
    pub playlist: String,
    /// URL path to retrieve the M3U8 playlist
    #[schema(example = "/api/v1/video/550e8400-e29b-41d4-a716-446655440000/playlist")]
    pub playlist_url: String,
    /// Local file path where the M3U8 was saved
    #[schema(example = "./playlists/550e8400-e29b-41d4-a716-446655440000.m3u8")]
    pub playlist_file: String,
}

/// Current status of a processing job.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Upload received, processing starting
    Pending,
    /// FFmpeg is slicing the video
    Processing,
    /// Segments are being uploaded to CDN
    Uploading,
    /// Everything completed successfully
    Completed,
    /// An error occurred
    Failed,
}

/// Stored result of a completed job.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobResult {
    /// Unique job identifier
    pub job_id: String,
    /// Final job status
    pub status: JobStatus,
    /// Final M3U8 playlist content
    pub playlist: String,
    /// Local file path where the M3U8 was saved
    pub playlist_file: String,
    /// Number of segments
    pub segments_count: usize,
    /// Number of uploaded segments
    pub segments_uploaded: usize,
}



/// Health check response.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Service status
    #[schema(example = "ok")]
    pub status: String,
    /// Service version
    #[schema(example = "0.1.0")]
    pub version: String,
    /// Whether FFmpeg is available
    pub ffmpeg_available: bool,
}

/// Response for listing all processed jobs.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobListResponse {
    /// List of job summaries
    pub jobs: Vec<JobSummary>,
    /// Total number of jobs
    pub total: usize,
}

/// Summary of a job (for listing).
#[derive(Debug, Serialize, ToSchema)]
pub struct JobSummary {
    /// Unique job identifier
    pub job_id: String,
    /// Current status
    pub status: JobStatus,
    /// Number of segments
    pub segments_count: usize,
    /// Number uploaded
    pub segments_uploaded: usize,
}

/// Multipart form data for video upload (used by Swagger UI).
#[derive(Debug, ToSchema)]
#[allow(dead_code)]
pub struct VideoUploadRequest {
    /// The video file to upload
    #[schema(value_type = String, format = Binary)]
    pub video: Vec<u8>,
    /// Optional HLS segment duration in seconds (default: 5)
    #[schema(example = "5", value_type = Option<String>)]
    pub segment_duration: Option<u32>,
}

/// Multipart form data for image upload (used by Swagger UI).
#[derive(Debug, ToSchema)]
#[allow(dead_code)]
pub struct ImageUploadRequest {
    /// One or more image files to upload
    #[schema(value_type = Vec<String>, format = Binary)]
    pub images: Vec<Vec<u8>>,
}

/// Response returned when image(s) are uploaded.
#[derive(Debug, Serialize, ToSchema)]
pub struct ImageUploadResponse {
    /// Total number of images submitted
    #[schema(example = 3)]
    pub total: usize,
    /// Number of images uploaded successfully
    #[schema(example = 3)]
    pub uploaded: usize,
    /// Number of images that failed to upload
    #[schema(example = 0)]
    pub failed: usize,
    /// Per-image upload results
    pub results: Vec<ImageResult>,
}

/// Result of uploading a single image.
#[derive(Debug, Serialize, ToSchema)]
pub struct ImageResult {
    /// Original filename
    #[schema(example = "photo.jpg")]
    pub filename: String,
    /// Remote CDN URL (present on success)
    #[schema(example = "https://cdn.example.com/image/photo.jpg")]
    pub url: Option<String>,
    /// Error message (present on failure)
    pub error: Option<String>,
}

// ──────────────────────────────────────────────
//  Internal models (not exposed via Swagger)
// ──────────────────────────────────────────────

/// Represents a single HLS segment on disk.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HlsSegment {
    /// Filename (e.g., "index0.ts")
    pub filename: String,
    /// Full path on disk
    pub path: std::path::PathBuf,
    /// Raw size in bytes
    pub size: u64,
}

/// Result of uploading a single segment to the CDN.
#[derive(Debug, Clone)]
pub struct SegmentUploadResult {
    /// Original local filename
    pub filename: String,
    /// Remote CDN URL
    pub remote_url: String,
    /// Original TS size (for BYTERANGE)
    pub original_size: u64,
}

/// CDN upload response structure (matches TikTok ads API).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CdnUploadResponse {
    pub code: Option<i32>,
    pub data: Option<CdnUploadData>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CdnUploadData {
    pub url: Option<String>,
}
