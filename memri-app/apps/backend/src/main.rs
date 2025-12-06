use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

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
use memri_capture::{start_capture, CaptureConfig};
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

    let monitors = if app_config.monitor_ids.is_empty() {
        vec![app_config.monitor_id]
    } else {
        app_config.monitor_ids.clone()
    };
    let (events_tx, _events_rx) = broadcast::channel::<String>(64);
    let storage = Arc::new(SqliteSink::from_app_config(&app_config).await?);
    let ocr_engine: Arc<dyn OcrEngine> = Arc::new(WindowsOcr);
    let anthropic = AnthropicClient::from_env();
    let api_key = env::var("MEMRI_API_KEY").ok();

    let notifying_sink: Arc<dyn memri_storage::CaptureSink> = Arc::new(NotifyingSink {
        inner: storage.clone(),
        tx: events_tx.clone(),
    });

    let mut capture_handles = Vec::new();
    for monitor_id in monitors {
        let cfg = CaptureConfig::from_app_config(&app_config, monitor_id);
        let handle = start_capture(cfg, ocr_engine.clone(), notifying_sink.clone()).await?;
        capture_handles.push(handle);
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
    let limit = params.limit.unwrap_or(20).min(200) as i64;
    state
        .storage
        .fetch_recent_captures(limit)
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

    let resp = client
        .request_stream(
            &history,
            &input.prompt,
            input.model.clone(),
            input.max_tokens,
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let stream = resp.bytes_stream().filter_map(|item| async move {
        match item {
            Ok(bytes) => {
                let chunk = String::from_utf8_lossy(&bytes).to_string();
                Some(Ok(Event::default().data(chunk)))
            }
            Err(err) => {
                error!("anthropic stream error: {err}");
                None
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
        let max_tokens = max_tokens.unwrap_or(256);

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
        let max_tokens = max_tokens.unwrap_or(256);

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

fn time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
