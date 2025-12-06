use anyhow::{anyhow, Context, Result};
use tracing::warn;
use image::{DynamicImage, ImageBuffer, Rgba};
use std::sync::Arc;
use tokio::task;
use xcap::Monitor;

#[derive(Clone)]
#[allow(dead_code)]
pub struct SafeMonitor {
    monitor_id: u32,
    data: Arc<MonitorData>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct MonitorData {
    pub width: u32,
    pub height: u32,
    pub name: String,
    pub is_primary: bool,
}

impl SafeMonitor {
    pub fn new(monitor: Monitor) -> Result<Self> {
        let monitor_id = monitor.id().context("missing monitor id")?;
        let width = monitor.width().context("missing monitor width")?;
        let height = monitor.height().context("missing monitor height")?;
        let name = monitor.name().unwrap_or_default().to_string();
        let is_primary = monitor.is_primary().unwrap_or(false);

        Ok(Self {
            monitor_id,
            data: Arc::new(MonitorData {
                width,
                height,
                name,
                is_primary,
            }),
        })
    }

    pub async fn capture_image(&self) -> Result<DynamicImage> {
        let monitor_id = self.monitor_id;
        task::spawn_blocking(move || -> Result<DynamicImage> {
            let monitor = Monitor::all()
                .map_err(anyhow::Error::from)?
                .into_iter()
                .find(|m| m.id().unwrap_or_default() == monitor_id)
                .ok_or_else(|| anyhow!("monitor {monitor_id} not found"))?;

            let width = monitor.width().context("monitor width missing")?;
            let height = monitor.height().context("monitor height missing")?;
            if width == 0 || height == 0 {
                return Err(anyhow!("monitor {monitor_id} has invalid dimensions"));
            }

            let buffer = monitor.capture_image().map_err(anyhow::Error::from)?;

            let rgba = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, buffer.into_raw())
                .ok_or_else(|| anyhow!("failed to convert monitor buffer"))?;

            Ok(DynamicImage::ImageRgba8(rgba))
        })
        .await?
    }

    pub fn id(&self) -> u32 {
        self.monitor_id
    }

    #[allow(dead_code)]
    pub fn width(&self) -> u32 {
        self.data.width
    }

    #[allow(dead_code)]
    pub fn height(&self) -> u32 {
        self.data.height
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.data.name
    }

    #[allow(dead_code)]
    pub fn is_primary(&self) -> bool {
        self.data.is_primary
    }
}

#[allow(dead_code)]
pub async fn get_monitor_by_id(id: u32) -> Result<SafeMonitor> {
    task::spawn_blocking(move || -> Result<SafeMonitor> {
        let mut monitors: Vec<Monitor> = Monitor::all().map_err(anyhow::Error::from)?.into_iter().collect();

        // Try the requested id first.
        if let Some(found) = monitors
            .iter()
            .find(|m| m.id().unwrap_or_default() == id)
            .cloned()
        {
            return SafeMonitor::new(found);
        }

        // Fallback: use the first available monitor (often the primary).
        if let Some(fallback) = monitors.pop() {
            warn!("monitor {id} not found, falling back to primary monitor");
            return SafeMonitor::new(fallback);
        }

        Err(anyhow!("no monitors detected"))
    })
    .await?
}

#[allow(dead_code)]
pub async fn list_monitors() -> Result<Vec<SafeMonitor>> {
    task::spawn_blocking(|| -> Result<Vec<SafeMonitor>> {
        Monitor::all()
            .map_err(anyhow::Error::from)?
            .into_iter()
            .map(SafeMonitor::new)
            .collect()
    })
    .await?
}
