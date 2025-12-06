//! Application-wide configuration helpers.
//!
//! Reads environment variables (with optional `.env`) and provides strongly
//! typed config structs consumed by other crates.

use std::env;

use anyhow::{Context, Result};

pub const DEFAULT_DATABASE_URL: &str = "sqlite://memri.db";
pub const DEFAULT_LANGUAGES: &str = "en";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub monitor_id: u32,
    pub monitor_ids: Vec<u32>,
    pub capture_interval_ms: u64,
    /// Maximum interval after backoff when no significant changes are detected.
    pub capture_max_interval_ms: u64,
    pub capture_unfocused_windows: bool,
    pub languages: Vec<String>,
    pub database_url: String,
    pub window_include: Vec<String>,
    pub window_ignore: Vec<String>,
    /// Retention window in days for capture records (0 disables).
    pub retention_days: u64,
    /// Maximum number of capture rows to keep (0 disables).
    pub max_captures: u64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let monitor_id = read_env_u32("MEMRI_MONITOR_ID", 0)?;
        let monitor_ids = read_env_list_u32("MEMRI_MONITOR_IDS");
        let capture_interval_ms = read_env_u64("MEMRI_CAPTURE_INTERVAL_MS", 2000)?;
        // Default to a 4x backoff window unless explicitly overridden.
        let capture_max_interval_ms =
            read_env_u64("MEMRI_CAPTURE_MAX_INTERVAL_MS", capture_interval_ms * 4)?;
        let capture_unfocused_windows = read_env_bool("MEMRI_CAPTURE_UNFOCUSED", false)?;
        let languages = read_env_list("MEMRI_LANGUAGES", DEFAULT_LANGUAGES);
        let database_url =
            env::var("MEMRI_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
        let window_include = read_env_list("MEMRI_WINDOW_INCLUDE", "");
        let window_ignore = read_env_list("MEMRI_WINDOW_IGNORE", "");
        let retention_days = read_env_u64("MEMRI_RETENTION_DAYS", 30)?;
        let max_captures = read_env_u64("MEMRI_MAX_CAPTURES", 5_000)?;

        Ok(Self {
            monitor_id,
            monitor_ids,
            capture_interval_ms,
            capture_max_interval_ms,
            capture_unfocused_windows,
            languages,
            database_url,
            window_include,
            window_ignore,
            retention_days,
            max_captures,
        })
    }
}

fn read_env_u32(key: &str, default: u32) -> Result<u32> {
    match env::var(key) {
        Ok(val) => val
            .parse::<u32>()
            .with_context(|| format!("Failed to parse {key} as u32")),
        Err(_) => Ok(default),
    }
}

fn read_env_u64(key: &str, default: u64) -> Result<u64> {
    match env::var(key) {
        Ok(val) => val
            .parse::<u64>()
            .with_context(|| format!("Failed to parse {key} as u64")),
        Err(_) => Ok(default),
    }
}

fn read_env_bool(key: &str, default: bool) -> Result<bool> {
    match env::var(key) {
        Ok(val) => match val.to_lowercase().as_str() {
            "1" | "true" | "yes" => Ok(true),
            "0" | "false" | "no" => Ok(false),
            other => Err(anyhow::anyhow!("Invalid boolean for {key}: {other}")),
        },
        Err(_) => Ok(default),
    }
}

fn read_env_list(key: &str, default: &str) -> Vec<String> {
    env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn read_env_list_u32(key: &str) -> Vec<u32> {
    env::var(key)
        .ok()
        .map(|v| {
            v.split(',')
                .filter_map(|s| s.trim().parse::<u32>().ok())
                .collect()
        })
        .unwrap_or_default()
}
