mod config;
mod errors;
mod handlers;
mod models;
mod services;

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::AppConfig;
use crate::handlers::{health, image, video};

/// OpenAPI documentation definition.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Video HLS API",
        version = "0.1.0",
        description = "REST API for uploading videos, slicing them into HLS segments with FFmpeg, \
                       masking segments with a 1×1 PNG prefix, uploading to a CDN, and generating \
                       M3U8 playlists with BYTERANGE tags for playback.",
        license(name = "MIT"),
        contact(name = "HieuCuteDangYeu")
    ),
    servers(
        (url = "/", description = "Current server")
    ),
    tags(
        (name = "Health", description = "Service health checks"),
        (name = "Video", description = "Video upload, processing, and playback"),
        (name = "Image", description = "Image upload to CDN")
    ),
    paths(
        health::health_check,
        video::upload_video,
        video::get_playlist,
        video::list_jobs,
        video::delete_job,
        image::upload_images,
    ),
    components(schemas(
        models::HealthResponse,
        models::UploadResponse,
        models::VideoUploadRequest,
        models::ImageUploadRequest,
        models::ImageUploadResponse,
        models::ImageResult,
        models::JobStatus,
        models::JobListResponse,
        models::JobSummary,
        errors::ErrorResponse,
    ))
)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = AppConfig::from_env();
    let bind_addr = format!("{}:{}", config.host, config.port);

    // Shared application state
    let app_state = web::Data::new(video::AppState {
        config: config.clone(),
        http_client: reqwest::Client::new(),
        jobs: Mutex::new(HashMap::new()),
    });

    info!("🚀 Starting Video HLS API on {}", bind_addr);
    info!(
        "📖 Swagger UI available at http://{}/swagger-ui/",
        bind_addr
    );

    // Ensure working directories exist
    std::fs::create_dir_all(&config.work_dir)?;
    std::fs::create_dir_all(&config.playlists_dir)?;

    HttpServer::new(move || {
        // CORS — allow everything for development
        let cors = Cors::permissive();

        App::new()
            // Shared state
            .app_data(app_state.clone())
            // Middleware
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap(tracing_actix_web::TracingLogger::default())
            // Max payload size
            .app_data(
                web::PayloadConfig::default()
                    .limit(config.max_upload_size_mb as usize * 1024 * 1024),
            )
            .app_data(
                actix_multipart::form::MultipartFormConfig::default()
                    .total_limit(config.max_upload_size_mb as usize * 1024 * 1024),
            )
            // Swagger UI
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            // API routes
            .service(health::health_check)
            .service(video::upload_video)
            .service(video::get_playlist)
            .service(video::list_jobs)
            .service(video::delete_job)
            .service(image::upload_images)
    })
    .bind(&bind_addr)?
    .run()
    .await
}
