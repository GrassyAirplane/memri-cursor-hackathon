//! Capture pipeline inspired by Screenpipe's continuous capture loop.
//!
//! The goal is to encapsulate monitor/window capture, change detection, and
//! dispatching work items downstream for OCR and storage.

mod platform;
mod monitor;
mod window_capture;

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
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
}

impl CaptureConfig {
    pub fn from_app_config(app: &AppConfig) -> Self {
        Self {
            monitor_id: app.monitor_id,
            interval: Duration::from_millis(app.capture_interval_ms),
            capture_unfocused_windows: app.capture_unfocused_windows,
            languages: app.languages.clone(),
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

    let raw_capture = match platform::capture_frame(config.monitor_id).await {
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

    let batch = CaptureBatch {
        frame_number,
        timestamp_ms,
        windows: vec![CapturedWindowRecord {
            window_name: "stub window".to_string(),
            app_name: "memri-app".to_string(),
            text: "OCR not yet implemented".to_string(),
            confidence: None,
            image_base64: None,
            browser_url: None,
        }],
    };

    sink.persist_batch(batch).await?;

    let _ = raw_capture;

    Ok(true)
}
