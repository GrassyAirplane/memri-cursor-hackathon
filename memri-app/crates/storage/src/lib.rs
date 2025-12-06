//! SQLite persistence layer for captures, OCR outputs, and conversations.
//!
//! Uses sqlx for async database access with Tokio.

use anyhow::Result;
use async_trait::async_trait;
use memri_config::AppConfig;
use serde::Serialize;
use sqlx::{
    sqlite::{SqlitePoolOptions, SqliteQueryResult},
    FromRow, Pool, QueryBuilder, Sqlite,
};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Incoming capture batch containing summary information.
#[derive(Debug, Clone, Serialize)]
pub struct CaptureBatch {
    pub frame_number: u64,
    pub timestamp_ms: i64,
    pub windows: Vec<CapturedWindowRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapturedWindowRecord {
    pub window_name: String,
    pub app_name: String,
    pub text: String,
    pub confidence: Option<f32>,
    pub ocr_json: Option<String>,
    pub image_base64: Option<String>,
    pub image_path: Option<String>,
    pub browser_url: Option<String>,
}

/// Capture with inlined windows, convenient for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct CaptureWithWindows {
    pub capture_id: i64,
    pub frame_number: i64,
    pub timestamp_ms: i64,
    pub windows: Vec<CapturedWindowRecord>,
}

/// Simple chat record model.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ChatMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at_ms: i64,
}

#[async_trait]
pub trait CaptureSink: Send + Sync {
    async fn persist_batch(&self, batch: CaptureBatch) -> Result<()>;
}

/// Concrete SQLite-backed sink.
pub struct SqliteSink {
    pool: Pool<Sqlite>,
    retention_days: Option<u64>,
    max_captures: Option<u64>,
}

impl SqliteSink {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let sink = Self {
            pool,
            retention_days: None,
            max_captures: None,
        };
        sink.run_migrations().await?;
        Ok(sink)
    }

    pub async fn from_app_config(config: &AppConfig) -> Result<Self> {
        let mut sink = Self::connect(&config.database_url).await?;
        sink.retention_days = if config.retention_days == 0 {
            None
        } else {
            Some(config.retention_days)
        };
        sink.max_captures = if config.max_captures == 0 {
            None
        } else {
            Some(config.max_captures)
        };
        Ok(sink)
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
                ocr_json TEXT,
                image_base64 TEXT,
                image_path TEXT,
                browser_url TEXT
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Best-effort migration for older databases without `ocr_json`.
        let _ = sqlx::query("ALTER TABLE captured_windows ADD COLUMN ocr_json TEXT")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("ALTER TABLE captured_windows ADD COLUMN image_path TEXT")
            .execute(&self.pool)
            .await;

        // Chat history storage.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS chat_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL DEFAULT (strftime('%s','now') * 1000)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Indices to accelerate common lookups.
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_captures_timestamp
            ON captures(timestamp_ms);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_windows_capture_id
            ON captured_windows(capture_id);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_chat_created_at
            ON chat_messages(created_at_ms);
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

        let insert_result: SqliteQueryResult =
            sqlx::query("INSERT INTO captures (frame_number, timestamp_ms) VALUES (?, ?)")
                .bind(batch.frame_number as i64)
                .bind(batch.timestamp_ms)
                .execute(&mut *conn)
                .await?;

        let capture_id = insert_result.last_insert_rowid();

        for window in batch.windows.iter() {
            sqlx::query(
                r#"INSERT INTO captured_windows (
                    capture_id, window_name, app_name, text, confidence, ocr_json, image_base64, image_path, browser_url
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(capture_id)
            .bind(window.window_name.clone())
            .bind(window.app_name.clone())
            .bind(window.text.clone())
            .bind(window.confidence)
            .bind(window.ocr_json.clone())
            .bind(window.image_base64.clone())
            .bind(window.image_path.clone())
            .bind(window.browser_url.clone())
            .execute(&mut *conn)
            .await?;
        }

        if let Err(err) = self.prune().await {
            warn!("pruning failed: {err}");
        }

        info!(
            frame = batch.frame_number,
            windows = batch.windows.len(),
            "persisted capture"
        );
        Ok(())
    }
}

impl SqliteSink {
    /// Persist a chat message for later retrieval.
    pub async fn insert_chat_message(&self, role: &str, content: &str) -> Result<i64> {
        let result = sqlx::query(r#"INSERT INTO chat_messages (role, content) VALUES (?, ?)"#)
            .bind(role)
            .bind(content)
            .execute(&self.pool)
            .await?;

        Ok(result.last_insert_rowid())
    }

    /// Fetch recent captures with their associated windows, ordered by newest first.
    pub async fn fetch_recent_captures(&self, limit: i64) -> Result<Vec<CaptureWithWindows>> {
        let limited = limit.max(0);
        if limited == 0 {
            return Ok(Vec::new());
        }

        let capture_rows: Vec<CaptureRow> = sqlx::query_as(
            r#"
            SELECT id, frame_number, timestamp_ms
            FROM captures
            ORDER BY timestamp_ms DESC
            LIMIT ?
            "#,
        )
        .bind(limited)
        .fetch_all(&self.pool)
        .await?;

        if capture_rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut captures: BTreeMap<i64, CaptureWithWindows> = capture_rows
            .into_iter()
            .map(|row| {
                (
                    row.id,
                    CaptureWithWindows {
                        capture_id: row.id,
                        frame_number: row.frame_number,
                        timestamp_ms: row.timestamp_ms,
                        windows: Vec::new(),
                    },
                )
            })
            .collect();

        let ids: Vec<i64> = captures.keys().copied().collect();
        let window_rows = fetch_windows_for_ids(&self.pool, &ids).await?;

        for row in window_rows {
            if let Some(capture) = captures.get_mut(&row.capture_id) {
                capture.windows.push(CapturedWindowRecord {
                    window_name: row.window_name.unwrap_or_default(),
                    app_name: row.app_name.unwrap_or_default(),
                    text: row.text.unwrap_or_default(),
                    confidence: row.confidence,
                    ocr_json: row.ocr_json,
                    image_base64: row.image_base64,
                    image_path: None,
                    browser_url: row.browser_url,
                });
            }
        }

        let mut ordered: Vec<CaptureWithWindows> = captures.into_values().collect();
        ordered.sort_by_key(|c| -c.timestamp_ms);
        Ok(ordered)
    }

    /// Fetch recent chat messages ordered newest first.
    pub async fn fetch_chat_messages(&self, limit: i64) -> Result<Vec<ChatMessage>> {
        let limited = limit.max(0);
        if limited == 0 {
            return Ok(Vec::new());
        }

        let rows: Vec<ChatMessage> = sqlx::query_as(
            r#"
            SELECT id, role, content, created_at_ms
            FROM chat_messages
            ORDER BY created_at_ms DESC
            LIMIT ?
            "#,
        )
        .bind(limited)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

#[derive(FromRow)]
struct CapturedWindowRow {
    capture_id: i64,
    window_name: Option<String>,
    app_name: Option<String>,
    text: Option<String>,
    confidence: Option<f32>,
    ocr_json: Option<String>,
    image_base64: Option<String>,
    browser_url: Option<String>,
}

#[derive(FromRow)]
struct CaptureRow {
    id: i64,
    frame_number: i64,
    timestamp_ms: i64,
}

impl SqliteSink {
    async fn prune(&self) -> Result<()> {
        if let Some(days) = self.retention_days {
            let cutoff_ms = current_time_ms().saturating_sub(days.saturating_mul(86_400_000));
            sqlx::query("DELETE FROM captures WHERE timestamp_ms < ?")
                .bind(cutoff_ms as i64)
                .execute(&self.pool)
                .await?;
        }

        if let Some(max) = self.max_captures {
            let total: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM captures")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

            let max_i64 = max as i64;
            if total > max_i64 {
                let to_delete = total - max_i64;
                sqlx::query(
                    r#"
                    DELETE FROM captures
                    WHERE id IN (
                        SELECT id FROM captures
                        ORDER BY timestamp_ms ASC
                        LIMIT ?
                    )
                    "#,
                )
                .bind(to_delete)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_millis(0))
        .as_millis() as u64
}

async fn fetch_windows_for_ids(pool: &Pool<Sqlite>, ids: &[i64]) -> Result<Vec<CapturedWindowRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::new(
        "SELECT capture_id, window_name, app_name, text, confidence, ocr_json, image_base64, browser_url FROM captured_windows WHERE capture_id IN (",
    );
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id);
    }
    builder.push(")");

    let query = builder.build_query_as::<CapturedWindowRow>();
    let rows = query.fetch_all(pool).await?;
    Ok(rows)
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
