//! SQLite persistence layer for captures, OCR outputs, and conversations.
//!
//! Uses sqlx for async database access with Tokio.

use anyhow::Result;
use async_trait::async_trait;
use memri_config::AppConfig;
use sqlx::{
    sqlite::{SqlitePoolOptions, SqliteQueryResult},
    Pool, Sqlite,
};
use tokio::sync::RwLock;
use tracing::info;

/// Incoming capture batch containing summary information.
#[derive(Debug, Clone)]
pub struct CaptureBatch {
    pub frame_number: u64,
    pub timestamp_ms: i64,
    pub windows: Vec<CapturedWindowRecord>,
}

#[derive(Debug, Clone)]
pub struct CapturedWindowRecord {
    pub window_name: String,
    pub app_name: String,
    pub text: String,
    pub confidence: Option<f32>,
    pub image_base64: Option<String>,
    pub browser_url: Option<String>,
}

#[async_trait]
pub trait CaptureSink: Send + Sync {
    async fn persist_batch(&self, batch: CaptureBatch) -> Result<()>;
}

/// Concrete SQLite-backed sink.
pub struct SqliteSink {
    pool: Pool<Sqlite>,
}

impl SqliteSink {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let sink = Self { pool };
        sink.run_migrations().await?;
        Ok(sink)
    }

    pub async fn from_app_config(config: &AppConfig) -> Result<Self> {
        Self::connect(&config.database_url).await
    }

    async fn run_migrations(&self) -> Result<()> {
        // Placeholder migration. Real migrations will create normalized tables.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS captures (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                frame_number INTEGER NOT NULL,
                timestamp_ms INTEGER NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS captured_windows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                capture_id INTEGER NOT NULL REFERENCES captures(id) ON DELETE CASCADE,
                window_name TEXT,
                app_name TEXT,
                text TEXT,
                confidence REAL,
                image_base64 TEXT,
                browser_url TEXT
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl CaptureSink for SqliteSink {
    async fn persist_batch(&self, batch: CaptureBatch) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let insert_result: SqliteQueryResult = sqlx::query(
            "INSERT INTO captures (frame_number, timestamp_ms) VALUES (?, ?)",
        )
        .bind(batch.frame_number as i64)
        .bind(batch.timestamp_ms)
        .execute(&mut *conn)
        .await?;

        let capture_id = insert_result.last_insert_rowid();

        for window in batch.windows {
            sqlx::query(
                r#"INSERT INTO captured_windows (
                    capture_id, window_name, app_name, text, confidence, image_base64, browser_url
                ) VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(capture_id)
            .bind(window.window_name)
            .bind(window.app_name)
            .bind(window.text)
            .bind(window.confidence)
            .bind(window.image_base64)
            .bind(window.browser_url)
            .execute(&mut *conn)
            .await?;
        }

        info!(frame = batch.frame_number, "persisted capture");
        Ok(())
    }
}

/// Shared handle wrapper useful for dependency injection.
pub struct SharedSink {
    inner: RwLock<Box<dyn CaptureSink>>,
}

impl SharedSink {
    pub fn new(inner: Box<dyn CaptureSink>) -> Self {
        Self {
            inner: RwLock::new(inner),
        }
    }

    pub async fn persist(&self, batch: CaptureBatch) -> Result<()> {
        let guard = self.inner.read().await;
        guard.persist_batch(batch).await
    }
}
