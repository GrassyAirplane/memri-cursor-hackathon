//! Capture pipeline inspired by Screenpipe's continuous capture loop.
//!
//! The goal is to encapsulate monitor/window capture, change detection, and
//! dispatching work items downstream for OCR and storage.

mod change_detection;
pub mod monitor;
mod platform;
mod window_capture;

use std::cmp;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use change_detection::{ChangeDecision, ChangeDetector};
use image::{DynamicImage, ImageFormat};
use memri_config::AppConfig;
use memri_ocr::{OcrContext, OcrEngine};
use memri_storage::{CaptureBatch, CaptureSink, CapturedWindowRecord};
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::sync::mpsc;
use tracing::{debug, info, instrument, warn};
use window_capture::WindowFilters;

/// Configuration values for starting the capture service.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub monitor_id: u32,
    pub interval: Duration,
    pub max_interval: Duration,
    pub capture_unfocused_windows: bool,
    pub languages: Vec<String>,
    pub window_include: Vec<String>,
    pub window_ignore: Vec<String>,
    pub image_dir: PathBuf,
}

impl CaptureConfig {
    pub fn from_app_config(app: &AppConfig, monitor_id: u32) -> Self {
        Self {
            monitor_id,
            interval: Duration::from_millis(app.capture_interval_ms),
            max_interval: Duration::from_millis(app.capture_max_interval_ms),
            capture_unfocused_windows: app.capture_unfocused_windows,
            languages: app.languages.clone(),
            window_include: app.window_include.clone(),
            window_ignore: app.window_ignore.clone(),
            image_dir: PathBuf::from(&app.image_dir),
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
    let mut backoff = Backoff::new(config.interval, config.max_interval);

    tokio::spawn(async move {
        let mut frame_number: u64 = 0;
        info!(monitor = config.monitor_id, "capture loop starting");
        let mut change_detector = ChangeDetector::new();
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("capture loop shutting down");
                    break;
                }
                _ = tokio::time::sleep(backoff.current_delay()) => {
                    debug!(monitor = config.monitor_id, delay_ms = backoff.current_delay().as_millis(), "tick");
                    match perform_iteration(&config, frame_number, &mut change_detector, ocr_engine.clone(), sink.clone()).await {
                        Ok(outcome) => {
                            backoff.record(&outcome.decision);
                            if outcome.captured {
                                frame_number = frame_number.saturating_add(1);
                            }
                        }
                        Err(err) => {
                            backoff.on_error();
                            warn!("capture iteration failed: {err}");
                        }
                    }
                }
            }
        }
    });

    Ok(CaptureHandle { shutdown_tx })
}

#[derive(Debug)]
struct IterationOutcome {
    decision: ChangeDecision,
    captured: bool,
}

#[instrument(skip(change_detector, ocr_engine, sink, config))]
async fn perform_iteration(
    config: &CaptureConfig,
    frame_number: u64,
    change_detector: &mut ChangeDetector,
    ocr_engine: Arc<dyn OcrEngine>,
    sink: Arc<dyn CaptureSink>,
) -> Result<IterationOutcome> {
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
            warn!(
                monitor = config.monitor_id,
                "platform capture failed: {err}"
            );
            return Err(err);
        }
    };

    let decision = change_detector.evaluate(&raw_capture.monitor_image);

    match decision {
        ChangeDecision::FirstFrame => {
            debug!(frame_number, "capturing baseline frame");
        }
        ChangeDecision::Significant {
            histogram_delta,
            ssim_score,
        } => {
            debug!(
                frame_number,
                histogram_delta, ssim_score, "significant change detected"
            );
        }
        ChangeDecision::Insignificant {
            histogram_delta,
            ssim_score,
        } => {
            debug!(
                frame_number,
                histogram_delta, ssim_score, "skipping frame without significant change"
            );
            return Ok(IterationOutcome {
                decision,
                captured: false,
            });
        }
    }

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_millis() as i64)
        .unwrap_or_default();

    let ocr_start = Instant::now();
    let windows = process_windows_for_ocr(
        &raw_capture.windows,
        &config.languages,
        ocr_engine,
        frame_number,
        timestamp_ms,
        &config.image_dir,
    )
    .await;
    let ocr_elapsed = ocr_start.elapsed();

    let batch = CaptureBatch {
        frame_number,
        timestamp_ms,
        windows,
    };

    let persist_start = Instant::now();
    sink.persist_batch(batch).await?;
    let persist_elapsed = persist_start.elapsed();

    debug!(
        frame_number,
        ocr_ms = ocr_elapsed.as_millis(),
        persist_ms = persist_elapsed.as_millis(),
        "capture iteration completed"
    );

    Ok(IterationOutcome {
        decision,
        captured: true,
    })
}

fn encode_image_png(image: &DynamicImage) -> Result<Vec<u8>, image::ImageError> {
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    image.write_to(&mut cursor, ImageFormat::Png)?;
    Ok(buffer)
}

#[instrument(skip(windows, ocr_engine, languages))]
async fn process_windows_for_ocr(
    windows: &[window_capture::CapturedWindow],
    languages: &[String],
    ocr_engine: Arc<dyn OcrEngine>,
    frame_number: u64,
    timestamp_ms: i64,
    image_dir: &Path,
) -> Vec<CapturedWindowRecord> {
    let mut records = Vec::with_capacity(windows.len());

    if let Err(err) = fs::create_dir_all(image_dir) {
        warn!("failed to create image_dir {:?}: {err}", image_dir);
    }

    let mut idx: usize = 0;
    for window in windows {
        let ocr_context = OcrContext {
            window_name: window.window_name.clone(),
            app_name: window.app_name.clone(),
            is_focused: window.is_focused,
            languages: languages.to_vec(),
        };

        let (png_bytes, image_path) =
            match save_image_to_disk(&window.image, image_dir, frame_number, timestamp_ms, idx) {
                Ok(val) => val,
                Err(err) => {
                    warn!(
                        window = window.window_name,
                        "failed to write window image: {err}"
                    );
                    records.push(CapturedWindowRecord {
                        window_name: window.window_name.clone(),
                        app_name: window.app_name.clone(),
                        text: String::new(),
                        confidence: None,
                        browser_url: None,
                        image_base64: None,
                        ocr_json: None,
                        image_path: None,
                    });
                    idx = idx.saturating_add(1);
                    continue;
                }
            };
        idx = idx.saturating_add(1);

        let ocr_result = ocr_engine
            .recognize(&png_bytes, &ocr_context)
            .await
            .map_err(|err| {
                warn!(
                    window = ocr_context.window_name,
                    engine = ocr_engine.name(),
                    "OCR failed: {err}"
                );
                err
            })
            .ok();

        let (text, confidence, ocr_json) = match ocr_result {
            Some(payload) => (payload.text, payload.confidence, payload.json),
            None => (String::new(), None, None),
        };

        records.push(CapturedWindowRecord {
            window_name: window.window_name.clone(),
            app_name: window.app_name.clone(),
            text,
            confidence,
            browser_url: extract_browser_url(
                window.is_focused,
                &window.app_name,
                &window.window_name,
            ),
            image_base64: None,
            ocr_json,
            image_path: Some(image_path),
        });
    }

    records
}

/// Simple exponential backoff used to throttle capture ticks after repeated
/// insignificant frames or transient errors.
#[derive(Debug, Clone)]
struct Backoff {
    base: Duration,
    max: Duration,
    current: Duration,
}

impl Backoff {
    fn new(base: Duration, max: Duration) -> Self {
        let clamped_max = cmp::max(max, base);
        Self {
            base,
            max: clamped_max,
            current: base,
        }
    }

    fn current_delay(&self) -> Duration {
        self.current
    }

    fn record(&mut self, decision: &ChangeDecision) {
        match decision {
            ChangeDecision::Significant { .. } | ChangeDecision::FirstFrame => {
                self.current = self.base;
            }
            ChangeDecision::Insignificant { .. } => {
                let next_ms = (self.current.as_millis() as f32 * 1.5).round() as u64;
                self.current = Duration::from_millis(next_ms).min(self.max);
            }
        }
    }

    fn on_error(&mut self) {
        let next = self.current + self.base;
        self.current = cmp::min(next, self.max);
    }
}

static URL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"https?://\S+").unwrap());

fn extract_browser_url(is_focused: bool, app_name: &str, window_title: &str) -> Option<String> {
    if !is_focused {
        return None;
    }

    let app = app_name.to_lowercase();
    let is_browser = [
        "chrome", "edge", "firefox", "brave", "opera", "vivaldi", "arc",
    ]
    .iter()
    .any(|needle| app.contains(needle));
    if !is_browser {
        return None;
    }

    let title = window_title;
    let m = URL_RE.find(title)?;
    let mut url = m.as_str().to_string();
    // Trim trailing punctuation often present in titles.
    while let Some(last) = url.chars().last() {
        if ",.;)]}>\"'".contains(last) {
            url.pop();
        } else {
            break;
        }
    }
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

fn save_image_to_disk(
    image: &DynamicImage,
    base_dir: &Path,
    frame_number: u64,
    timestamp_ms: i64,
    idx: usize,
) -> Result<(Vec<u8>, String), image::ImageError> {
    let png_bytes = encode_image_png(image)?;
    let filename = format!("frame_{}_{}_{}.png", timestamp_ms, frame_number, idx);
    let path = base_dir.join(filename);
    fs::write(&path, &png_bytes).map_err(image::ImageError::IoError)?;
    let path_str = path.to_string_lossy().to_string();
    Ok((png_bytes, path_str))
}
