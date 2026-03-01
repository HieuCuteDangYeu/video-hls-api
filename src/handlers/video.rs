use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpResponse};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;
use tracing::{info, error};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::{AppError, ErrorResponse};
use crate::models::*;
use crate::services::{ffmpeg, hls, upload};


/// Shared application state holding completed job results.
pub struct AppState {
    pub config: AppConfig,
    pub http_client: reqwest::Client,
    pub jobs: Mutex<HashMap<String, JobResult>>,
}

/// Upload a video file, slice into HLS segments, upload to CDN, and return the playlist.
///
/// Send the video as a `multipart/form-data` request with the field name `video`.
/// Optionally include a `segment_duration` field (integer, seconds).
#[utoipa::path(
    post,
    path = "/api/v1/video/upload",
    tag = "Video",
    request_body(
        content = VideoUploadRequest,
        content_type = "multipart/form-data",
        description = "Upload a video file. Field name must be `video`. Optionally include `segment_duration` (integer).",
    ),
    responses(
        (status = 200, description = "Video processed and uploaded successfully", body = UploadResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 502, description = "CDN upload failed", body = ErrorResponse),
        (status = 500, description = "Internal processing error", body = ErrorResponse),
    )
)]
#[post("/api/v1/video/upload")]
pub async fn upload_video(
    state: web::Data<AppState>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let job_id = Uuid::new_v4();
    let job_dir = std::path::PathBuf::from(&state.config.work_dir).join(job_id.to_string());
    std::fs::create_dir_all(&job_dir)?;

    let mut video_path: Option<std::path::PathBuf> = None;
    let mut segment_duration: Option<u32> = None;

    // ── Parse multipart fields ──────────────────────────────────
    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| AppError::MultipartError(e.to_string()))?;

        let field_name = field
            .content_disposition()
            .and_then(|cd| cd.get_name().map(|s| s.to_string()))
            .unwrap_or_default();

        match field_name.as_str() {
            "video" => {
                let filename = field
                    .content_disposition()
                    .and_then(|cd| cd.get_filename().map(|s| s.to_string()))
                    .unwrap_or_else(|| "input.mp4".to_string());

                let dest = job_dir.join(&filename);
                let mut file = std::fs::File::create(&dest)?;

                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| AppError::MultipartError(e.to_string()))?;
                    file.write_all(&chunk)?;
                }

                info!("Received video file: {} → {:?}", filename, dest);
                video_path = Some(dest);
            }
            "segment_duration" => {
                let mut buf = Vec::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| AppError::MultipartError(e.to_string()))?;
                    buf.extend_from_slice(&chunk);
                }
                if let Ok(s) = String::from_utf8(buf) {
                    segment_duration = s.trim().parse().ok();
                }
            }
            _ => {
                // Drain unknown fields
                while field.next().await.is_some() {}
            }
        }
    }

    let video_path = video_path.ok_or_else(|| {
        AppError::ValidationError("Missing required field: `video`".to_string())
    })?;

    let seg_dur = segment_duration.unwrap_or(state.config.hls_segment_duration);
    if !(1..=30).contains(&seg_dur) {
        return Err(AppError::ValidationError(
            "segment_duration must be between 1 and 30".to_string(),
        ));
    }

    // ── Step 1: FFmpeg slicing ──────────────────────────────────
    info!("🎬 Job {}: Slicing video ({}s segments)…", job_id, seg_dur);
    let hls_dir = job_dir.join("hls");
    let segments = web::block({
        let video_path = video_path.clone();
        let hls_dir = hls_dir.clone();
        move || ffmpeg::slice_video(&video_path, &hls_dir, seg_dur)
    })
    .await
    .map_err(|e| AppError::InternalError(e.to_string()))??;

    let segments_count = segments.len();
    info!("Produced {} segments", segments_count);

    // ── Step 2: Upload segments to CDN ──────────────────────────
    info!("🚀 Job {}: Uploading {} segments…", job_id, segments_count);
    let upload_results =
        upload::upload_all_segments(&state.http_client, &state.config, &segments).await;
    let segments_uploaded = upload_results.len();
    info!("Uploaded {}/{} segments", segments_uploaded, segments_count);

    if upload_results.is_empty() && segments_count > 0 {
        return Err(AppError::UploadError(
            "All segment uploads failed — check CDN credentials".to_string(),
        ));
    }

    // ── Step 3: Rewrite playlist ────────────────────────────────
    info!("🛠️ Job {}: Rewriting M3U8 playlist…", job_id);
    let playlist_file = ffmpeg::playlist_path(&hls_dir);
    let final_playlist = hls::rewrite_playlist(&playlist_file, &upload_results)?;

    // ── Step 4: Save M3U8 locally ───────────────────────────────
    let playlists_dir = std::path::PathBuf::from(&state.config.playlists_dir);
    let saved_path = hls::save_playlist(&playlists_dir, &job_id.to_string(), &final_playlist)?;
    let playlist_file_str = saved_path.to_string_lossy().to_string();
    info!("📁 Job {}: Playlist saved to {}", job_id, playlist_file_str);

    // ── Store result ────────────────────────────────────────────
    let job_result = JobResult {
        job_id: job_id.to_string(),
        status: JobStatus::Completed,
        playlist: final_playlist.clone(),
        playlist_file: playlist_file_str.clone(),
        segments_count,
        segments_uploaded,
    };

    state
        .jobs
        .lock()
        .unwrap()
        .insert(job_id.to_string(), job_result);

    // ── Cleanup temp files (best-effort) ────────────────────────
    if let Err(e) = std::fs::remove_dir_all(&job_dir) {
        error!("Cleanup failed for {:?}: {}", job_dir, e);
    }

    let response = UploadResponse {
        job_id: job_id.to_string(),
        status: JobStatus::Completed,
        segments_count,
        segments_uploaded,
        playlist: final_playlist,
        playlist_url: format!("/api/v1/video/{}/playlist", job_id),
        playlist_file: playlist_file_str,
    };

    info!("🎉 Job {} completed successfully", job_id);
    Ok(HttpResponse::Ok().json(response))
}

/// Retrieve the M3U8 playlist for a completed job.
#[utoipa::path(
    get,
    path = "/api/v1/video/{job_id}/playlist",
    tag = "Video",
    params(
        ("job_id" = String, Path, description = "The job UUID returned from the upload endpoint")
    ),
    responses(
        (status = 200, description = "M3U8 playlist content", content_type = "application/x-mpegURL"),
        (status = 404, description = "Job not found", body = ErrorResponse),
    )
)]
#[get("/api/v1/video/{job_id}/playlist")]
pub async fn get_playlist(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let job_id = path.into_inner();
    let jobs = state.jobs.lock().unwrap();
    let job = jobs
        .get(&job_id)
        .ok_or_else(|| AppError::NotFound(format!("Job {} not found", job_id)))?;

    Ok(HttpResponse::Ok()
        .content_type("application/x-mpegURL")
        .body(job.playlist.clone()))
}

/// List all completed jobs.
#[utoipa::path(
    get,
    path = "/api/v1/video/jobs",
    tag = "Video",
    responses(
        (status = 200, description = "List of all jobs", body = JobListResponse),
    )
)]
#[get("/api/v1/video/jobs")]
pub async fn list_jobs(state: web::Data<AppState>) -> HttpResponse {
    let jobs = state.jobs.lock().unwrap();
    let summaries: Vec<JobSummary> = jobs
        .values()
        .map(|j| JobSummary {
            job_id: j.job_id.clone(),
            status: j.status.clone(),
            segments_count: j.segments_count,
            segments_uploaded: j.segments_uploaded,
        })
        .collect();

    let total = summaries.len();
    HttpResponse::Ok().json(JobListResponse {
        jobs: summaries,
        total,
    })
}

/// Delete a completed job from memory.
#[utoipa::path(
    delete,
    path = "/api/v1/video/{job_id}",
    tag = "Video",
    params(
        ("job_id" = String, Path, description = "The job UUID to delete")
    ),
    responses(
        (status = 204, description = "Job deleted successfully"),
        (status = 404, description = "Job not found", body = ErrorResponse),
    )
)]
#[actix_web::delete("/api/v1/video/{job_id}")]
pub async fn delete_job(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let job_id = path.into_inner();
    let mut jobs = state.jobs.lock().unwrap();

    if jobs.remove(&job_id).is_some() {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Err(AppError::NotFound(format!("Job {} not found", job_id)))
    }
}
