use std::collections::HashSet;

use anyhow::{Error, Result};
use image::{DynamicImage, ImageBuffer};
use once_cell::sync::Lazy;
use tracing::{debug, error, trace};
use xcap::Window;

use crate::monitor::SafeMonitor;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CapturedWindow {
    pub image: DynamicImage,
    pub app_name: String,
    pub window_name: String,
    pub process_id: i32,
    pub is_focused: bool,
}

#[derive(Debug, Clone)]
pub struct WindowFilters {
    ignore_set: HashSet<String>,
    include_set: HashSet<String>,
}

impl WindowFilters {
    pub fn new(ignore_list: &[String], include_list: &[String]) -> Self {
        Self {
            ignore_set: ignore_list.iter().map(|s| s.to_lowercase()).collect(),
            include_set: include_list.iter().map(|s| s.to_lowercase()).collect(),
        }
    }

    pub fn is_valid(&self, app_name: &str, title: &str) -> bool {
        let app_name_lower = app_name.to_lowercase();
        let title_lower = title.to_lowercase();

        if !self.ignore_set.is_empty()
            && self
                .ignore_set
                .iter()
                .any(|ignore| app_name_lower.contains(ignore) || title_lower.contains(ignore))
        {
            return false;
        }

        if self.include_set.is_empty() {
            return true;
        }

        self.include_set.iter().any(|include| {
            app_name_lower.contains(include) || title_lower.contains(include)
        })
    }
}

#[cfg(target_os = "windows")]
static SKIP_APPS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "Windows Shell Experience Host",
        "Microsoft Text Input Application",
        "Windows Explorer",
        "Program Manager",
        "Microsoft Store",
        "Search",
        "TaskBar",
    ])
});

#[cfg(not(target_os = "windows"))]
static SKIP_APPS: Lazy<HashSet<&'static str>> = Lazy::new(HashSet::new);

#[cfg(target_os = "windows")]
static SKIP_TITLES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "Program Manager",
        "Windows Input Experience",
        "Microsoft Text Input Application",
        "Task View",
        "Start",
        "System Tray",
        "Notification Area",
        "Action Center",
        "Task Bar",
        "Desktop",
    ])
});

#[cfg(not(target_os = "windows"))]
static SKIP_TITLES: Lazy<HashSet<&'static str>> = Lazy::new(HashSet::new);

pub async fn capture_all_visible_windows(
    monitor: &SafeMonitor,
    window_filters: &WindowFilters,
    capture_unfocused_windows: bool,
) -> Result<Vec<CapturedWindow>> {
    let windows = Window::all().map_err(Error::from)?;

    if windows.is_empty() {
        return Ok(Vec::new());
    }

    let mut captured = Vec::new();
    let monitor_id = monitor.id();
    trace!(monitor_id, "processing visible windows");
    for window in windows {
        let app_name = match window.app_name() {
            Ok(name) => name.to_string(),
            Err(err) => {
                debug!("skipping window without app name: {err}");
                continue;
            }
        };

        let title = match window.title() {
            Ok(title) => title.to_string(),
            Err(err) => {
                error!("failed to get window title for {app_name}: {err}");
                continue;
            }
        };

        if SKIP_APPS.contains(app_name.as_str()) || SKIP_TITLES.contains(title.as_str()) {
            debug!("skipping known system window");
            continue;
        }

        if !window_filters.is_valid(&app_name, &title) {
            continue;
        }

        let is_minimized = window.is_minimized().unwrap_or(true);
        if is_minimized {
            debug!("skipping minimized window {app_name} ({title})");
            continue;
        }

        let is_focused = window.is_focused().unwrap_or(false);
        if !capture_unfocused_windows && !is_focused {
            continue;
        }

        let process_id = window.pid().unwrap_or_default() as i32;

        let buffer = match window.capture_image() {
            Ok(buffer) => buffer,
            Err(err) => {
                error!("failed to capture window image for {app_name}: {err}");
                continue;
            }
        };

        let rgba = match ImageBuffer::from_raw(buffer.width(), buffer.height(), buffer.into_raw()) {
            Some(buffer) => buffer,
            None => {
                error!("invalid buffer size for window {app_name}");
                continue;
            }
        };
        let image = DynamicImage::ImageRgba8(rgba);

        captured.push(CapturedWindow {
            image,
            app_name,
            window_name: title,
            process_id,
            is_focused,
        });
    }

    Ok(captured)
}
