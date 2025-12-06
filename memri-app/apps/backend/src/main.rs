use std::sync::Arc;

use anyhow::Result;
use memri_capture::{start_capture, CaptureConfig};
use memri_config::AppConfig;
use memri_ocr::{OcrEngine, WindowsOcr};
use memri_storage::SqliteSink;
use tokio::signal;
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let app_config = AppConfig::from_env()?;
    info!(?app_config, "loaded configuration");

    let capture_config = CaptureConfig::from_app_config(&app_config);
    let storage = Arc::new(SqliteSink::from_app_config(&app_config).await?);
    let ocr_engine: Arc<dyn OcrEngine> = Arc::new(WindowsOcr);

    let handle = start_capture(capture_config, ocr_engine, storage).await?;

    signal::ctrl_c().await?;
    handle.shutdown().await;

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,memri_capture=debug"));

    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_max_level(Level::TRACE)
        .init();
}
