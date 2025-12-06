use std::{env, fs, path::Path};

use anyhow::Result;
use serde::Deserialize;

/// Configuration loaded from `memri-config.toml` (or `memri.config.toml`) at repo root.
/// All fields are optional; if present they will populate environment variables
/// consumed by the rest of the app to avoid `.env` usage.
#[derive(Debug, Default, Deserialize)]
pub struct FileConfig {
    #[serde(default)]
    pub app: AppSection,
    #[serde(default)]
    pub api: ApiSection,
}

#[derive(Debug, Default, Deserialize)]
pub struct AppSection {
    pub monitor_id: Option<u32>,
    pub monitor_ids: Option<Vec<u32>>,
    pub capture_interval_ms: Option<u64>,
    pub capture_max_interval_ms: Option<u64>,
    pub capture_unfocused_windows: Option<bool>,
    pub languages: Option<Vec<String>>,
    pub database_url: Option<String>,
    pub window_include: Option<Vec<String>>,
    pub window_ignore: Option<Vec<String>>,
    pub retention_days: Option<u64>,
    pub max_captures: Option<u64>,
    pub image_dir: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ApiSection {
    pub addr: Option<String>,
    pub key: Option<String>,
    pub anthropic_api_key: Option<String>,
}

const CANDIDATES: &[&str] = &["memri-config.toml", "memri.config.toml", "config/memri-config.toml"];

pub fn load_file_config_into_env() -> Result<()> {
    let cfg = read_first_config()?;
    if let Some(cfg) = cfg {
        set_if_missing("MEMRI_MONITOR_ID", cfg.app.monitor_id.map(|v| v.to_string()));
        set_if_missing(
            "MEMRI_MONITOR_IDS",
            cfg.app
                .monitor_ids
                .map(|v| v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")),
        );
        set_if_missing(
            "MEMRI_CAPTURE_INTERVAL_MS",
            cfg.app.capture_interval_ms.map(|v| v.to_string()),
        );
        set_if_missing(
            "MEMRI_CAPTURE_MAX_INTERVAL_MS",
            cfg.app.capture_max_interval_ms.map(|v| v.to_string()),
        );
        set_if_missing(
            "MEMRI_CAPTURE_UNFOCUSED",
            cfg.app
                .capture_unfocused_windows
                .map(|v| if v { "true".into() } else { "false".into() }),
        );
        set_if_missing(
            "MEMRI_LANGUAGES",
            cfg.app
                .languages
                .map(|v| v.join(",")),
        );
        set_if_missing("MEMRI_DATABASE_URL", cfg.app.database_url);
        set_if_missing(
            "MEMRI_WINDOW_INCLUDE",
            cfg.app.window_include.map(|v| v.join(",")),
        );
        set_if_missing(
            "MEMRI_WINDOW_IGNORE",
            cfg.app.window_ignore.map(|v| v.join(",")),
        );
        set_if_missing("MEMRI_RETENTION_DAYS", cfg.app.retention_days.map(|v| v.to_string()));
        set_if_missing("MEMRI_MAX_CAPTURES", cfg.app.max_captures.map(|v| v.to_string()));
        set_if_missing("MEMRI_IMAGE_DIR", cfg.app.image_dir);

        // API-specific vars used by backend startup.
        set_if_missing("MEMRI_API_ADDR", cfg.api.addr);
        set_if_missing("MEMRI_API_KEY", cfg.api.key);
        set_if_missing("ANTHROPIC_API_KEY", cfg.api.anthropic_api_key);
    }
    Ok(())
}

fn read_first_config() -> Result<Option<FileConfig>> {
    for candidate in CANDIDATES {
        let path = Path::new(candidate);
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let cfg: FileConfig = toml::from_str(&contents)?;
            return Ok(Some(cfg));
        }
    }
    Ok(None)
}

fn set_if_missing(key: &str, val: Option<String>) {
    if let Some(val) = val {
        let trimmed = val.trim();
        if trimmed.is_empty() {
            return;
        }
        if env::var(key).is_err() {
            env::set_var(key, trimmed);
        }
    }
}

