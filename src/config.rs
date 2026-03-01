use std::env;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Host to bind the server to
    pub host: String,
    /// Port to bind the server to
    pub port: u16,
    /// Directory for temporary HLS processing
    pub work_dir: String,
    /// Directory for saving final M3U8 playlists
    pub playlists_dir: String,
    /// CDN upload endpoint URL
    pub cdn_upload_endpoint: String,
    /// CSRF token for CDN requests
    pub cdn_csrf_token: String,
    /// UUID header for CDN requests
    pub cdn_uuid: String,
    /// Cookie header for CDN requests
    pub cdn_cookie: String,
    /// HLS segment duration in seconds
    pub hls_segment_duration: u32,
    /// Maximum video upload file size in MB
    pub max_upload_size_mb: u64,
    /// Maximum image upload total size in MB
    pub max_image_upload_size_mb: u64,
}

impl AppConfig {
    /// Load configuration from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        Self {
            host: env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("APP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            work_dir: env::var("WORK_DIR").unwrap_or_else(|_| "./hls_work".to_string()),
            playlists_dir: env::var("PLAYLISTS_DIR").unwrap_or_else(|_| "./playlists".to_string()),
            cdn_upload_endpoint: env::var("CDN_UPLOAD_ENDPOINT")
                .expect("CDN_UPLOAD_ENDPOINT must be set"),
            cdn_csrf_token: env::var("CDN_CSRF_TOKEN")
                .expect("CDN_CSRF_TOKEN must be set"),
            cdn_uuid: env::var("CDN_UUID")
                .expect("CDN_UUID must be set"),
            cdn_cookie: env::var("CDN_COOKIE")
                .expect("CDN_COOKIE must be set"),
            hls_segment_duration: env::var("HLS_SEGMENT_DURATION")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            max_upload_size_mb: env::var("MAX_UPLOAD_SIZE_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(500),
            max_image_upload_size_mb: env::var("MAX_IMAGE_UPLOAD_SIZE_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50),
        }
    }
}
