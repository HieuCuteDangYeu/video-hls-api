use std::env;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub work_dir: String,
    pub playlists_dir: String,
    pub cdn_upload_endpoint: String,
    pub cdn_csrf_token: String,
    pub cdn_uuid: String,
    pub cdn_cookie: String,
    pub hls_segment_duration: u32,
    pub max_upload_size_mb: u64,
    pub max_image_upload_size_mb: u64,
}

impl AppConfig {
    /// Load configuration from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        // Helper to fetch an env var and strip literal quotes injected by Docker
        let clean_var = |key: &str| -> String {
            env::var(key)
                .unwrap_or_else(|_| panic!("{} must be set", key))
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        };

        Self {
            host: env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("APP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            work_dir: env::var("WORK_DIR").unwrap_or_else(|_| "./hls_work".to_string()),
            playlists_dir: env::var("PLAYLISTS_DIR").unwrap_or_else(|_| "./playlists".to_string()),
            
            // Use the helper for strings that might be quoted in the .env file
            cdn_upload_endpoint: clean_var("CDN_UPLOAD_ENDPOINT"),
            cdn_csrf_token: clean_var("CDN_CSRF_TOKEN"),
            cdn_uuid: clean_var("CDN_UUID"),
            cdn_cookie: clean_var("CDN_COOKIE"),
            
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