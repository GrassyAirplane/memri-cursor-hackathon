use anyhow::Result;
#[cfg(not(target_os = "windows"))]
use anyhow::anyhow;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct RawCapture;

#[cfg(target_os = "windows")]
pub async fn capture_frame(monitor_id: u32) -> Result<RawCapture> {
    debug!(monitor_id, "capturing frame via Windows APIs (stub)");
    // TODO: integrate real Windows capture similar to Screenpipe's monitor pipeline.
    Ok(RawCapture)
}

#[cfg(target_os = "macos")]
pub async fn capture_frame(monitor_id: u32) -> Result<RawCapture> {
    debug!(monitor_id, "capturing frame via macOS APIs (stub)");
    Err(anyhow!("macOS capture not implemented yet"))
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub async fn capture_frame(monitor_id: u32) -> Result<RawCapture> {
    debug!(monitor_id, "capture request on unsupported platform");
    Err(anyhow!("platform capture not implemented for this OS"))
}
