//! SQLite persistence layer for captures, OCR outputs, and conversations.
//!
//! Uses sqlx for async database access with Tokio.

use anyhow::Result;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use memri_config::AppConfig;
use serde::Serialize;
use sqlx::{
    sqlite::{SqlitePoolOptions, SqliteQueryResult},
    FromRow, Pool, QueryBuilder, Sqlite,
};
use std::collections::{BTreeMap, HashMap};
use std::fs;
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
                // Load image from disk if image_path exists but no base64
                let image_base64 = if row.image_base64.is_some() {
                    row.image_base64
                } else if let Some(ref path) = row.image_path {
                    load_image_as_base64(path)
                } else {
                    None
                };

                capture.windows.push(CapturedWindowRecord {
                    window_name: row.window_name.unwrap_or_default(),
                    app_name: row.app_name.unwrap_or_default(),
                    text: row.text.unwrap_or_default(),
                    confidence: row.confidence,
                    ocr_json: row.ocr_json,
                    image_base64,
                    image_path: row.image_path,
                    browser_url: row.browser_url,
                });
            }
        }

        let mut ordered: Vec<CaptureWithWindows> = captures.into_values().collect();
        ordered.sort_by_key(|c| -c.timestamp_ms);
        Ok(ordered)
    }

    /// Fetch captures metadata WITHOUT loading images (fast for initial load).
    pub async fn fetch_captures_metadata(&self, limit: i64) -> Result<Vec<CaptureWithWindows>> {
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
        // Fetch windows but skip image loading
        let window_rows = fetch_windows_metadata_for_ids(&self.pool, &ids).await?;

        for row in window_rows {
            if let Some(capture) = captures.get_mut(&row.capture_id) {
                capture.windows.push(CapturedWindowRecord {
                    window_name: row.window_name.unwrap_or_default(),
                    app_name: row.app_name.unwrap_or_default(),
                    text: row.text.unwrap_or_default(),
                    confidence: row.confidence,
                    ocr_json: row.ocr_json,
                    image_base64: None, // Don't load images
                    image_path: row.image_path,
                    browser_url: row.browser_url,
                });
            }
        }

        let mut ordered: Vec<CaptureWithWindows> = captures.into_values().collect();
        ordered.sort_by_key(|c| -c.timestamp_ms);
        Ok(ordered)
    }

    /// Fetch images for specific capture IDs.
    pub async fn fetch_images_for_captures(&self, ids: &[i64]) -> Result<HashMap<i64, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut builder = QueryBuilder::new(
            "SELECT capture_id, image_path FROM captured_windows WHERE capture_id IN (",
        );
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id);
        }
        builder.push(")");

        let query = builder.build_query_as::<ImagePathRow>();
        let rows: Vec<ImagePathRow> = query.fetch_all(&self.pool).await?;

        let mut images = HashMap::new();
        for row in rows {
            if let Some(path) = row.image_path {
                if let Some(base64) = load_image_as_base64(&path) {
                    images.insert(row.capture_id, base64);
                }
            }
        }

        Ok(images)
    }

    /// Search captures by text content and/or time range.
    /// Returns captures where OCR text, window name, app name, or browser URL matches ANY of the query terms.
    pub async fn search_captures(
        &self,
        query: &str,
        start_time_ms: Option<i64>,
        end_time_ms: Option<i64>,
        limit: i64,
    ) -> Result<Vec<CaptureWithWindows>> {
        // Split query into individual terms and search for each
        let terms: Vec<&str> = query.split_whitespace().filter(|t| t.len() > 1).collect();
        
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        
        // Build dynamic SQL with OR conditions for each term
        let mut where_clauses = Vec::new();
        let mut bindings = Vec::new();
        
        for term in &terms {
            let pattern = format!("%{}%", term.to_lowercase());
            where_clauses.push("(LOWER(cw.text) LIKE ? OR LOWER(cw.window_name) LIKE ? OR LOWER(cw.app_name) LIKE ? OR LOWER(cw.browser_url) LIKE ?)");
            // Each clause needs 4 bindings
            for _ in 0..4 {
                bindings.push(pattern.clone());
            }
        }
        
        let where_clause = where_clauses.join(" OR ");
        
        let time_clause = if start_time_ms.is_some() || end_time_ms.is_some() {
            " AND (? IS NULL OR c.timestamp_ms >= ?) AND (? IS NULL OR c.timestamp_ms <= ?)"
        } else {
            ""
        };
        
        let sql = format!(
            r#"
            SELECT DISTINCT c.id, c.frame_number, c.timestamp_ms
            FROM captures c
            JOIN captured_windows cw ON cw.capture_id = c.id
            WHERE ({}){}
            ORDER BY c.timestamp_ms DESC
            LIMIT ?
            "#,
            where_clause,
            time_clause
        );

        let mut query_builder = sqlx::query_as::<_, CaptureRow>(&sql);
        
        // Bind all the search patterns
        for pattern in &bindings {
            query_builder = query_builder.bind(pattern);
        }
        
        // Bind time constraints if present
        if start_time_ms.is_some() || end_time_ms.is_some() {
            query_builder = query_builder
                .bind(start_time_ms)
                .bind(start_time_ms)
                .bind(end_time_ms)
                .bind(end_time_ms);
        }
        
        // Bind limit
        query_builder = query_builder.bind(limit);
        
        let capture_rows: Vec<CaptureRow> = query_builder.fetch_all(&self.pool).await?;

        if capture_rows.is_empty() {
            return Ok(Vec::new());
        }

        let ids: Vec<i64> = capture_rows.iter().map(|r| r.id).collect();
        let window_rows = fetch_windows_metadata_for_ids(&self.pool, &ids).await?;

        let mut by_capture: BTreeMap<i64, CaptureWithWindows> = capture_rows
            .into_iter()
            .map(|c| {
                (
                    c.id,
                    CaptureWithWindows {
                        capture_id: c.id,
                        frame_number: c.frame_number,
                        timestamp_ms: c.timestamp_ms,
                        windows: vec![],
                    },
                )
            })
            .collect();

        for wr in window_rows {
            if let Some(capture) = by_capture.get_mut(&wr.capture_id) {
                capture.windows.push(CapturedWindowRecord {
                    window_name: wr.window_name.unwrap_or_default(),
                    app_name: wr.app_name.unwrap_or_default(),
                    text: wr.text.unwrap_or_default(),
                    confidence: wr.confidence,
                    ocr_json: wr.ocr_json,
                    image_base64: None,
                    image_path: wr.image_path,
                    browser_url: wr.browser_url,
                });
            }
        }

        Ok(by_capture.into_values().collect())
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
    image_path: Option<String>,
    browser_url: Option<String>,
}

#[derive(FromRow)]
struct CaptureRow {
    id: i64,
    frame_number: i64,
    timestamp_ms: i64,
}

#[derive(FromRow)]
struct ImagePathRow {
    capture_id: i64,
    image_path: Option<String>,
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

/// Load an image file from disk and encode it as base64.
fn load_image_as_base64(path: &str) -> Option<String> {
    match fs::read(path) {
        Ok(bytes) => Some(BASE64.encode(&bytes)),
        Err(_) => None,
    }
}

async fn fetch_windows_for_ids(pool: &Pool<Sqlite>, ids: &[i64]) -> Result<Vec<CapturedWindowRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::new(
        "SELECT capture_id, window_name, app_name, text, confidence, ocr_json, image_base64, image_path, browser_url FROM captured_windows WHERE capture_id IN (",
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

/// Fetch windows metadata WITHOUT loading images (for fast list loading).
async fn fetch_windows_metadata_for_ids(pool: &Pool<Sqlite>, ids: &[i64]) -> Result<Vec<CapturedWindowRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder = QueryBuilder::new(
        "SELECT capture_id, window_name, app_name, text, confidence, ocr_json, NULL as image_base64, image_path, browser_url FROM captured_windows WHERE capture_id IN (",
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
