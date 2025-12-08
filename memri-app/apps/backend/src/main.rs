use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::{fs, io};

use anyhow::Result;
use async_trait::async_trait;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Json, Router,
};
use futures_util::{Stream, StreamExt};
use memri_capture::{monitor::list_monitors, start_capture, CaptureConfig};
use memri_config::AppConfig;
use memri_ocr::{OcrEngine, WindowsOcr};
use memri_storage::{CaptureWithWindows, ChatMessage, SqliteSink};
use serde::Deserialize;
use serde::Serialize;
use tokio::signal;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;
use tracing::{error, info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let app_config = AppConfig::from_env()?;
    info!(?app_config, "loaded configuration");

    let capture_disabled = env::var("MEMRI_DISABLE_CAPTURE")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);

    let (events_tx, _events_rx) = broadcast::channel::<String>(64);
    let storage = Arc::new(SqliteSink::from_app_config(&app_config).await?);
    let ocr_engine: Arc<dyn OcrEngine> = Arc::new(WindowsOcr);
    let anthropic = AnthropicClient::from_env();
    let api_key = env::var("MEMRI_API_KEY").ok();

    // Optional seeding from a static captures folder for headless environments.
    let seed_dir = env::var("MEMRI_SEED_CAPTURE_DIR").unwrap_or_else(|_| "captures-seed".into());
    if let Err(err) = seed_captures_from_dir(storage.clone(), &seed_dir).await {
        error!(%seed_dir, ?err, "failed to seed captures from directory");
    }

    let notifying_sink: Arc<dyn memri_storage::CaptureSink> = Arc::new(NotifyingSink {
        inner: storage.clone(),
        tx: events_tx.clone(),
    });

    let mut capture_handles = Vec::new();

    if capture_disabled {
        info!("MEMRI_DISABLE_CAPTURE is set; skipping screen capture and OCR");
    } else {
        // Discover available monitors up front and reconcile config.
        let available = list_monitors().await.unwrap_or_default();
        if available.is_empty() {
            error!("no monitors detected on this system; capture cannot start");
            return Ok(());
        }

        // Build the desired monitor list: use explicit monitor_ids if provided, otherwise the single monitor_id.
        let mut requested = if app_config.monitor_ids.is_empty() {
            vec![app_config.monitor_id]
        } else {
            app_config.monitor_ids.clone()
        };

        // Filter to available monitors; fallback to first available if none match.
        let available_ids: Vec<u32> = available.iter().map(|m| m.id()).collect();
        requested.retain(|id| available_ids.contains(id));
        if requested.is_empty() {
            let fallback = available_ids[0];
            error!(
                "configured monitor(s) not found: {:?}; falling back to primary monitor {}",
                if app_config.monitor_ids.is_empty() {
                    vec![app_config.monitor_id]
                } else {
                    app_config.monitor_ids.clone()
                },
                fallback
            );
            requested.push(fallback);
        }

        info!("available monitors: {:?}", available_ids);
        info!("using monitors: {:?}", requested);

        for monitor_id in requested {
            let cfg = CaptureConfig::from_app_config(&app_config, monitor_id);
            let handle = start_capture(cfg, ocr_engine.clone(), notifying_sink.clone()).await?;
            capture_handles.push(handle);
        }
    }

    let api_task = start_api_server(
        storage.clone(),
        events_tx.clone(),
        anthropic.clone(),
        api_key,
    );

    signal::ctrl_c().await?;
    for handle in capture_handles {
        handle.shutdown().await;
    }
    api_task.abort();

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

#[derive(Clone)]
struct AppState {
    storage: Arc<SqliteSink>,
    events_tx: broadcast::Sender<String>,
    anthropic: Option<AnthropicClient>,
}

fn start_api_server(
    storage: Arc<SqliteSink>,
    events_tx: broadcast::Sender<String>,
    anthropic: Option<AnthropicClient>,
    api_key: Option<String>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let state = AppState {
            storage,
            events_tx,
            anthropic,
        };
        let app = build_router(state, api_key);

        let addr: SocketAddr = env::var("MEMRI_API_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse()
            .expect("invalid MEMRI_API_ADDR");

        info!(%addr, "starting api server");

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("failed to bind api address");

        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("api server error: {err}");
        }
    })
}

fn build_router(state: AppState, api_key: Option<String>) -> Router {
    let cors = CorsLayer::permissive();
    Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route("/captures", get(list_captures))
        .route("/captures/images", get(get_capture_images))
        .route("/events", get(capture_events))
        .route("/chat", get(list_chat_messages).post(add_chat_message))
        .route("/assistant", get(list_chat_messages).post(run_assistant))
        .route(
            "/assistant/stream",
            get(list_chat_messages).post(stream_assistant),
        )
        .with_state(state)
        .layer(cors)
        .layer(middleware::from_fn_with_state(api_key, enforce_api_key))
}

async fn enforce_api_key(
    State(expected): State<Option<String>>,
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(expected_key) = expected {
        let provided = req.headers().get("x-api-key").and_then(|h| h.to_str().ok());
        if provided != Some(expected_key.as_str()) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(next.run(req).await)
}

#[derive(Deserialize)]
struct ListParams {
    limit: Option<u32>,
}

async fn list_captures(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<CaptureWithWindows>>, StatusCode> {
    // No limit by default - return all captures metadata (no images for performance).
    let limit = params.limit.map(|l| l as i64).unwrap_or(i64::MAX);
    state
        .storage
        .fetch_captures_metadata(limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
struct ImageParams {
    ids: String, // Comma-separated capture IDs
}

/// Fetch images for specific capture IDs (on-demand loading).
async fn get_capture_images(
    State(state): State<AppState>,
    Query(params): Query<ImageParams>,
) -> Result<Json<std::collections::HashMap<i64, String>>, StatusCode> {
    let ids: Vec<i64> = params
        .ids
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    
    if ids.is_empty() {
        return Ok(Json(std::collections::HashMap::new()));
    }
    
    state
        .storage
        .fetch_images_for_captures(&ids)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn list_chat_messages(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<ChatMessage>>, StatusCode> {
    let limit = params.limit.unwrap_or(50).min(500) as i64;
    state
        .storage
        .fetch_chat_messages(limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
struct ChatInput {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AssistantInput {
    prompt: String,
    max_tokens: Option<u32>,
    model: Option<String>,
}

async fn add_chat_message(
    State(state): State<AppState>,
    Json(input): Json<ChatInput>,
) -> Result<StatusCode, StatusCode> {
    if input.role.trim().is_empty() || input.content.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .storage
        .insert_chat_message(&input.role, &input.content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Fire-and-forget chat event; ignore send errors if no listeners.
    let _ = state.events_tx.send(
        serde_json::json!({
            "type": "chat",
            "role": input.role,
            "content": input.content,
        })
        .to_string(),
    );

    Ok(StatusCode::CREATED)
}

async fn run_assistant(
    State(state): State<AppState>,
    Json(input): Json<AssistantInput>,
) -> Result<Json<ChatMessage>, StatusCode> {
    let span = tracing::info_span!("assistant_request", model = %input.model.clone().unwrap_or_else(|| "claude-3-5-sonnet-latest".into()));
    let _guard = span.enter();

    let client = match &state.anthropic {
        Some(c) => c,
        None => return Err(StatusCode::SERVICE_UNAVAILABLE),
    };

    // Build context from recent messages (most recent first), trim to last 15.
    let mut history = state
        .storage
        .fetch_chat_messages(15)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    history.reverse(); // oldest first for LLM

    // Store user message in history & DB.
    state
        .storage
        .insert_chat_message("user", &input.prompt)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    history.push(ChatMessage {
        id: -1,
        role: "user".to_string(),
        content: input.prompt.clone(),
        created_at_ms: time_ms(),
    });

    let assistant_reply = client
        .send_message(
            &history,
            &input.prompt,
            input.model.clone(),
            input.max_tokens,
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let assistant_id = state
        .storage
        .insert_chat_message("assistant", &assistant_reply)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let chat_msg = ChatMessage {
        id: assistant_id,
        role: "assistant".to_string(),
        content: assistant_reply.clone(),
        created_at_ms: time_ms(),
    };

    // Emit event; ignore if no listeners.
    let _ = state.events_tx.send(
        serde_json::json!({
            "type": "chat",
            "role": "assistant",
            "content": assistant_reply,
        })
        .to_string(),
    );

    Ok(Json(chat_msg))
}

/// Parse time-related keywords from user query and return (start_time_ms, end_time_ms)
fn parse_time_range(query: &str) -> (Option<i64>, Option<i64>) {
    let query_lower = query.to_lowercase();
    let now_ms = time_ms();
    let day_ms: i64 = 86_400_000;
    let hour_ms: i64 = 3_600_000;
    
    if query_lower.contains("yesterday") {
        // Yesterday: start of yesterday to end of yesterday
        let start = now_ms - (2 * day_ms);
        let end = now_ms - day_ms;
        return (Some(start), Some(end));
    }
    
    if query_lower.contains("today") {
        // Today: start of today (midnight) to now
        let start = now_ms - day_ms;
        return (Some(start), None);
    }
    
    if query_lower.contains("last hour") || query_lower.contains("past hour") {
        let start = now_ms - hour_ms;
        return (Some(start), None);
    }
    
    if query_lower.contains("this morning") {
        // Assume morning is 6am-12pm today
        let start = now_ms - day_ms;
        return (Some(start), None);
    }
    
    if query_lower.contains("last week") || query_lower.contains("past week") {
        let start = now_ms - (7 * day_ms);
        return (Some(start), None);
    }
    
    // No specific time range detected
    (None, None)
}

/// Extract search terms from user query (apps, keywords, etc.)
fn extract_search_terms(query: &str) -> String {
    let query_lower = query.to_lowercase();
    
    let mut terms = Vec::new();
    
    // Stop words to exclude from search
    let stop_words = [
        "what", "did", "i", "do", "on", "the", "was", "were", "last", "yesterday", 
        "today", "show", "me", "find", "search", "look", "for", "my", "a", "an", "in",
        "have", "has", "been", "any", "some", "which", "where", "when", "how", "why",
        "can", "could", "would", "should", "will", "that", "this", "these", "those",
        "with", "from", "about", "into", "through", "during", "before", "after",
        "above", "below", "between", "under", "again", "further", "then", "once",
        "here", "there", "all", "each", "few", "more", "most", "other", "such",
        "only", "own", "same", "than", "too", "very", "just", "also", "now",
        "work", "done", "watched", "looked", "used", "opened", "saw", "see",
        "videos", "video", "page", "pages", "site", "sites", "app", "apps",
    ];
    
    // Extract quoted strings as exact search terms (highest priority)
    let mut in_quote = false;
    let mut current_term = String::new();
    for ch in query.chars() {
        if ch == '"' || ch == '\'' {
            if in_quote && !current_term.is_empty() {
                terms.push(current_term.clone());
                current_term.clear();
            }
            in_quote = !in_quote;
        } else if in_quote {
            current_term.push(ch);
        }
    }
    
    // Extract ALL meaningful words from query (not just if apps not found)
    for word in query_lower.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());
        if cleaned.len() > 2 && !stop_words.contains(&cleaned) && !terms.contains(&cleaned.to_string()) {
            terms.push(cleaned.to_string());
        }
    }
    
    // If still no terms, be very permissive - just grab anything 3+ chars
    if terms.is_empty() {
        for word in query_lower.split_whitespace() {
            let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());
            if cleaned.len() >= 3 {
                terms.push(cleaned.to_string());
                break; // Just get one term to search with
            }
        }
    }
    
    terms.join(" ")
}

/// Build an enhanced prompt with capture context for the LLM
fn build_context_prompt(original_query: &str, captures: &[memri_storage::CaptureWithWindows]) -> String {
    use chrono::{TimeZone, Utc};
    
    let mut context_parts = Vec::new();
    
    for cap in captures {
        let dt = Utc.timestamp_millis_opt(cap.timestamp_ms).unwrap();
        let timestamp = dt.format("%Y-%m-%d %H:%M:%S").to_string();
        
        for window in &cap.windows {
            let app = &window.app_name;
            let title = &window.window_name;
            let text_preview = if window.text.len() > 200 {
                format!("{}...", &window.text[..200])
            } else {
                window.text.clone()
            };
            
            context_parts.push(format!(
                "[[CLIP:{}]] At {}, App: {}, Window: \"{}\"\nContent: {}",
                cap.capture_id, timestamp, app, title, text_preview
            ));
        }
    }
    
    if context_parts.is_empty() {
        return original_query.to_string();
    }
    
    format!(
        r#"You are a helpful assistant with access to the user's screen captures. Here are relevant captures I found:

{}

IMPORTANT: When you mention ANY information from the captures above, you MUST include the exact clip marker [[CLIP:ID]] (replacing ID with the number) inline in your response. This creates a clickable link for the user.

Example response format:
"Based on your captures, you were watching a YouTube video [[CLIP:123]] about programming, and then you searched for Rust tutorials [[CLIP:124]]."

Now answer the user's question, making sure to include [[CLIP:ID]] markers for each capture you reference:

{}"#,
        context_parts.join("\n\n"),
        original_query
    )
}

async fn stream_assistant(
    State(state): State<AppState>,
    Json(input): Json<AssistantInput>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let span = tracing::info_span!("assistant_stream", model = %input.model.clone().unwrap_or_else(|| "claude-3-5-sonnet-latest".into()));
    let _guard = span.enter();

    let client = match &state.anthropic {
        Some(c) => c,
        None => return Err(StatusCode::SERVICE_UNAVAILABLE),
    };

    // Build context
    let mut history = state
        .storage
        .fetch_chat_messages(15)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    history.reverse();

    // Search captures for relevant context based on user query
    let (start_time, end_time) = parse_time_range(&input.prompt);
    let search_terms = extract_search_terms(&input.prompt);
    
    let relevant_captures = if !search_terms.is_empty() {
        state
            .storage
            .search_captures(&search_terms, start_time, end_time, 5)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Build enhanced prompt with capture context
    let enhanced_prompt = if relevant_captures.is_empty() {
        input.prompt.clone()
    } else {
        build_context_prompt(&input.prompt, &relevant_captures)
    };

    // Store user message in DB before streaming
    state
        .storage
        .insert_chat_message("user", &input.prompt)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let resp = client
        .request_stream(
            &history,
            &enhanced_prompt,
            input.model.clone(),
            input.max_tokens,
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Clone storage for use in stream
    let storage = state.storage.clone();
    let accumulated_text = Arc::new(tokio::sync::Mutex::new(String::new()));
    let accumulated_clone = accumulated_text.clone();

    let stream = resp.bytes_stream().filter_map(move |item| {
        let text_ref = accumulated_clone.clone();
        async move {
        match item {
            Ok(bytes) => {
                let chunk = String::from_utf8_lossy(&bytes).to_string();
                    
                    // Parse SSE format: extract text from content_block_delta events
                    let mut extracted_text = String::new();
                    for line in chunk.lines() {
                        if line.starts_with("data: ") {
                            let json_str = &line[6..]; // Skip "data: " prefix
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                                // Look for content_block_delta with text
                                if json["type"] == "content_block_delta" {
                                    if let Some(text) = json["delta"]["text"].as_str() {
                                        extracted_text.push_str(text);
                                    }
                                }
                            }
                        }
                    }
                    
                    if !extracted_text.is_empty() {
                        // Accumulate for saving later
                        let mut text = text_ref.lock().await;
                        text.push_str(&extracted_text);
                        Some(Ok(Event::default().data(extracted_text)))
                    } else {
                        None
                    }
            }
            Err(err) => {
                error!("anthropic stream error: {err}");
                None
                }
            }
        }
    });

    // Spawn task to save assistant response after stream completes
    // Use a longer delay to ensure the stream has finished
    tokio::spawn(async move {
        // Wait longer for stream to complete (complex responses can take time)
        tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
        let final_text = accumulated_text.lock().await;
        if !final_text.is_empty() {
            info!("Saving assistant message ({} chars)", final_text.len());
            if let Err(e) = storage.insert_chat_message("assistant", &final_text).await {
                error!("Failed to save assistant message: {e}");
            }
        }
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn capture_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| async move {
        match msg {
            Ok(payload) => Some(Ok(Event::default().data(payload))),
            Err(_) => None,
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

struct NotifyingSink {
    inner: Arc<SqliteSink>,
    tx: broadcast::Sender<String>,
}

#[async_trait]
impl memri_storage::CaptureSink for NotifyingSink {
    async fn persist_batch(&self, batch: memri_storage::CaptureBatch) -> Result<()> {
        self.inner.persist_batch(batch.clone()).await?;

        // Emit minimal event payload; ignore if no subscribers.
        let _ = self.tx.send(
            serde_json::json!({
                "type": "capture",
                "frame_number": batch.frame_number,
                "timestamp_ms": batch.timestamp_ms,
                "windows": batch.windows.len(),
            })
            .to_string(),
        );

        Ok(())
    }
}

#[derive(Clone)]
struct AnthropicClient {
    api_key: String,
    http: reqwest::Client,
}

impl AnthropicClient {
    fn from_env() -> Option<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY").ok()?;
        let http = reqwest::Client::new();
        Some(Self { api_key, http })
    }

    async fn send_message(
        &self,
        history: &[ChatMessage],
        prompt: &str,
        model: Option<String>,
        max_tokens: Option<u32>,
    ) -> Result<String> {
        let model = model.unwrap_or_else(|| "claude-3-5-sonnet-latest".to_string());
        let max_tokens = max_tokens.unwrap_or(2048);

        let mut messages: Vec<AnthropicMessage> = history
            .iter()
            .map(|m| AnthropicMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
        messages.push(AnthropicMessage {
            role: "user".into(),
            content: prompt.to_string(),
        });

        let body = AnthropicRequest {
            model,
            max_tokens,
            messages,
            stream: None,
        };

        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                error!("anthropic request failed: {err}");
                err
            })?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            error!("anthropic error: {status} {text}");
            return Err(anyhow::anyhow!("anthropic error"));
        }

        let parsed: AnthropicResponse = serde_json::from_str(&text).map_err(|err| {
            error!("anthropic parse failed: {err}");
            err
        })?;

        let reply = parsed
            .content
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        Ok(reply)
    }

    async fn request_stream(
        &self,
        history: &[ChatMessage],
        prompt: &str,
        model: Option<String>,
        max_tokens: Option<u32>,
    ) -> Result<reqwest::Response> {
        let model = model.unwrap_or_else(|| "claude-3-5-sonnet-latest".to_string());
        let max_tokens = max_tokens.unwrap_or(2048);

        let mut messages: Vec<AnthropicMessage> = history
            .iter()
            .map(|m| AnthropicMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
        messages.push(AnthropicMessage {
            role: "user".into(),
            content: prompt.to_string(),
        });

        let body = AnthropicRequest {
            model,
            max_tokens,
            messages,
            stream: Some(true),
        };

        self.http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                error!("anthropic stream request failed: {err}");
                anyhow::Error::from(err)
            })
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: Option<String>,
}

/// Seed the database from a static captures directory if the DB is empty.
async fn seed_captures_from_dir(storage: Arc<SqliteSink>, dir: &str) -> Result<()> {
    let path = Path::new(dir);
    if !path.exists() || !path.is_dir() {
        return Ok(()); // nothing to do
    }

    // Skip seeding if captures already exist.
    let existing = storage.fetch_captures_metadata(1).await?;
    if !existing.is_empty() {
        info!("database already has captures; skipping seed");
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.path());

    let mut count = 0u64;
    for (idx, entry) in entries.into_iter().enumerate() {
        let p = entry.path();
        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if !["png", "jpg", "jpeg", "webp", "bmp"].contains(&ext.as_str()) {
            continue;
        }

        let ts_ms = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or_else(|_| time_ms())
            })
            .unwrap_or_else(|_| time_ms());

        let batch = memri_storage::CaptureBatch {
            frame_number: idx as u64,
            timestamp_ms: ts_ms,
            windows: vec![memri_storage::CapturedWindowRecord {
                window_name: p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string(),
                app_name: "static".to_string(),
                text: String::new(),
                confidence: None,
                ocr_json: None,
                image_base64: None,
                image_path: Some(p.to_string_lossy().to_string()),
                browser_url: None,
            }],
        };

        storage.persist_batch(batch).await?;
        count += 1;
    }

    info!(seeded = count, dir, "seeded captures from static folder");
    Ok(())
}

fn time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
