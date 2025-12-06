//! Capture pipeline inspired by Screenpipe's continuous capture loop.
//!
//! The goal is to encapsulate monitor/window capture, change detection, and
//! dispatching work items downstream for OCR and storage.

mod platform;
mod monitor;
mod window_capture;

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::io::Cursor;

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat};
use memri_config::AppConfig;
use memri_ocr::OcrEngine;
use memri_storage::{CaptureBatch, CaptureSink, CapturedWindowRecord};
use window_capture::WindowFilters;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Configuration values for starting the capture service.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub monitor_id: u32,
    pub interval: Duration,
    pub capture_unfocused_windows: bool,
    pub languages: Vec<String>,
    pub window_include: Vec<String>,
    pub window_ignore: Vec<String>,
}

impl CaptureConfig {
    pub fn from_app_config(app: &AppConfig) -> Self {
        Self {
            monitor_id: app.monitor_id,
            interval: Duration::from_millis(app.capture_interval_ms),
            capture_unfocused_windows: app.capture_unfocused_windows,
            languages: app.languages.clone(),
            window_include: app.window_include.clone(),
            window_ignore: app.window_ignore.clone(),
        }
    }
}

/// Public handle that allows external components to request a graceful shutdown.
#[derive(Clone)]
pub struct CaptureHandle {
    shutdown_tx: mpsc::Sender<()>,
}

impl CaptureHandle {
    pub async fn shutdown(self) {
        if let Err(err) = self.shutdown_tx.send(()).await {
            warn!("capture shutdown channel closed: {err}");
        }
    }
}

/// Start the asynchronous capture loop.
pub async fn start_capture(
    config: CaptureConfig,
    ocr_engine: Arc<dyn OcrEngine>,
    sink: Arc<dyn CaptureSink>,
) -> Result<CaptureHandle> {
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    tokio::spawn(async move {
        let mut frame_number: u64 = 0;
        info!(monitor = config.monitor_id, "capture loop starting");
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("capture loop shutting down");
                    break;
                }
                _ = tokio::time::sleep(config.interval) => {
                    debug!(monitor = config.monitor_id, "tick");
                    match perform_iteration(&config, frame_number, ocr_engine.clone(), sink.clone()).await {
                        Ok(true) => {
                            frame_number = frame_number.saturating_add(1);
                        }
                        Ok(false) => {}
                        Err(err) => {
                        warn!("capture iteration failed: {err}");
                        }
                    }
                }
            }
        }
    });

    Ok(CaptureHandle { shutdown_tx })
}

async fn perform_iteration(
    config: &CaptureConfig,
    frame_number: u64,
    ocr_engine: Arc<dyn OcrEngine>,
    sink: Arc<dyn CaptureSink>,
) -> Result<bool> {
    // Placeholder for real monitor/window capture.
    // In the full implementation this will:
    // 1. Capture monitor + window thumbnails.
    // 2. Detect if the frame changed meaningfully.
    // 3. Dispatch OCR jobs and persist results.
    debug!(monitor = config.monitor_id, "performing capture iteration");

    let window_filters = WindowFilters::new(&config.window_ignore, &config.window_include);

    let raw_capture = match platform::capture_frame(
        config.monitor_id,
        config.capture_unfocused_windows,
        &window_filters,
    )
    .await
    {
        Ok(data) => data,
        Err(err) => {
            warn!(monitor = config.monitor_id, "platform capture failed: {err}");
            return Ok(false);
        }
    };

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_millis() as i64)
        .unwrap_or_default();

    debug!(engine = ocr_engine.name(), frame_number, "running OCR pipeline (stub)");

    let windows = raw_capture
        .windows
        .iter()
        .map(|window| CapturedWindowRecord {
            window_name: window.window_name.clone(),
            app_name: window.app_name.clone(),
            text: String::new(),
            confidence: None,
            image_base64: encode_image_base64(&window.image).ok(),
            browser_url: None,
        })
        .collect();

    let batch = CaptureBatch {
        frame_number,
        timestamp_ms,
        windows,
    };

    sink.persist_batch(batch).await?;

    let _ = raw_capture.monitor_image;

    Ok(true)
}

fn encode_image_base64(image: &DynamicImage) -> Result<String, image::ImageError> {
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    image.write_to(&mut cursor, ImageFormat::Png)?;
    Ok(general_purpose::STANDARD.encode(buffer))
}
