use anyhow::Result;
use image::DynamicImage;
use tracing::{debug, warn};

use crate::monitor::get_monitor_by_id;
use crate::window_capture::{capture_all_visible_windows, CapturedWindow, WindowFilters};

#[derive(Debug)]
pub struct RawCapture {
    pub monitor_image: DynamicImage,
    pub windows: Vec<CapturedWindow>,
}

#[cfg(target_os = "windows")]
pub async fn capture_frame(
    monitor_id: u32,
    capture_unfocused_windows: bool,
    window_filters: &WindowFilters,
) -> Result<RawCapture> {
    debug!(monitor_id, "capturing frame via Windows APIs");

    let monitor = get_monitor_by_id(monitor_id).await?;
    let monitor_image = monitor.capture_image().await?;

    let windows = match capture_all_visible_windows(
        &monitor,
        window_filters,
        capture_unfocused_windows,
    )
    .await
    {
        Ok(captured) => captured,
        Err(err) => {
            warn!(monitor_id, "failed to capture window set: {err}");
            Vec::new()
        }
    };

    Ok(RawCapture {
        monitor_image,
        windows,
    })
}

#[cfg(not(target_os = "windows"))]
pub async fn capture_frame(
    monitor_id: u32,
    _capture_unfocused_windows: bool,
    _window_filters: &WindowFilters,
) -> Result<RawCapture> {
    use anyhow::anyhow;
    debug!(monitor_id, "capture request on unsupported platform");
    Err(anyhow!("platform capture not implemented for this OS"))
}
