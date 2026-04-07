use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
};
use tracing::{info, warn};

const DEFAULT_TIMEOUT_MS: u64 = 120_000;
const DEFAULT_PROVIDER_ORDER: &[&str] = &["codex", "copilot"];

#[derive(Clone)]
struct AppState {
    adapters: HashMap<String, Arc<AdapterClient>>,
    provider_order: Vec<String>,
    base_url_hint: String,
    response_counter: Arc<AtomicU64>,
}

struct AdapterClient {
    name: &'static str,
    script_path: PathBuf,
    next_id: AtomicU64,
    restart_count: AtomicU64,
    last_restart_reason: Mutex<Option<String>>,
    process: Mutex<AdapterProcess>,
}

struct AdapterProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    result: Option<Value>,
    error: Option<AdapterError>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AdapterError {
    code: String,
    message: String,
    retryable: bool,
}

impl AdapterError {
    fn transport(message: impl Into<String>) -> Self {
        Self {
            code: "gateway_transport_error".to_string(),
            message: message.into(),
            retryable: false,
        }
    }

    fn is_transport(&self) -> bool {
        self.code == "gateway_transport_error"
    }
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AdapterError {}

#[derive(Debug, Deserialize)]
struct InitializeResult {
    adapter: String,
    protocol_version: u32,
    ready: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct AdapterHealth {
    ok: bool,
    adapter: String,
    logged_in: bool,
    models: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auth_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auth_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    login: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bootstrap_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    restart_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_restart_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdapterModels {
    models: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CompletionResult {
    provider: String,
    model: String,
    text: String,
    usage: Option<CompletionUsage>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CompletionUsage {
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Serialize, Clone)]
struct CompletionAttempt {
    provider: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retryable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

struct CompletionEnvelope {
    result: CompletionResult,
    attempts: Vec<CompletionAttempt>,
}

struct CompletionFailure {
    message: String,
    attempts: Vec<CompletionAttempt>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "patchhive_ai_local_gateway=info,info".to_string()),
        )
        .with_target(false)
        .compact()
        .init();

    let host = std::env::var("PATCHHIVE_AI_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PATCHHIVE_AI_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8787);

    let mut adapters = HashMap::new();
    adapters.insert("codex".to_string(), Arc::new(spawn_adapter("codex").await?));

    if env_bool("PATCHHIVE_AI_ENABLE_COPILOT", true) {
        match spawn_adapter("copilot").await {
            Ok(adapter) => {
                adapters.insert("copilot".to_string(), Arc::new(adapter));
            }
            Err(error) => warn!("failed to spawn copilot adapter: {error}"),
        }
    }

    let provider_order = resolved_provider_order(&adapters);
    let state = AppState {
        adapters,
        provider_order,
        base_url_hint: format!("http://{host}:{port}/v1"),
        response_counter: Arc::new(AtomicU64::new(1)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/responses", post(responses_api))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .with_context(|| format!("failed to bind Rust gateway on {host}:{port}"))?;

    info!("patchhive-ai-local-rust listening on http://{host}:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let mut providers = Map::new();
    let mut any_ok = false;

    for provider in ordered_adapter_names(&state) {
        let Some(adapter) = state.adapters.get(&provider) else {
            continue;
        };

        let provider_value = match adapter.health().await {
            Ok(health) => {
                any_ok |= health.ok;
                serde_json::to_value(health).unwrap_or_else(|_| {
                    json!({
                        "ok": false,
                        "adapter": provider,
                        "error": "failed to serialize adapter health",
                    })
                })
            }
            Err(error) => json!({
                "ok": false,
                "adapter": provider,
                "logged_in": false,
                "models": [],
                "error": error.to_string(),
                "restart_count": adapter.restart_count.load(Ordering::SeqCst),
                "last_restart_reason": adapter.last_restart_reason().await,
            }),
        };

        providers.insert(provider, provider_value);
    }

    Json(json!({
        "ok": any_ok,
        "gateway": "patchhive-ai-local-rust",
        "provider_order": state.provider_order,
        "providers": providers,
        "base_url_hint": state.base_url_hint,
    }))
    .into_response()
}

async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let mut data = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut errors = Vec::new();

    for provider in ordered_adapter_names(&state) {
        let Some(adapter) = state.adapters.get(&provider) else {
            continue;
        };

        match adapter.list_models().await {
            Ok(models) => {
                for id in models {
                    if seen_ids.insert(id.clone()) {
                        data.push(json!({
                            "id": id,
                            "object": "model",
                            "owned_by": format!("patchhive-{provider}"),
                        }));
                    }
                }
            }
            Err(error) => errors.push(format!("{provider}: {error}")),
        }
    }

    if data.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": {
                    "message": if errors.is_empty() {
                        "No adapters returned any models.".to_string()
                    } else {
                        format!("No adapters returned any models. {}", errors.join("; "))
                    },
                    "type": "patchhive_gateway_error",
                }
            })),
        )
            .into_response();
    }

    Json(json!({
        "object": "list",
        "data": data,
    }))
    .into_response()
}

async fn chat_completions(State(state): State<AppState>, Json(body): Json<Value>) -> impl IntoResponse {
    if body
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "message": "Streaming is not supported yet by the Rust gateway.",
                    "type": "patchhive_gateway_error",
                }
            })),
        )
            .into_response();
    }

    match complete_with_fallback(&state, &body, body.get("messages").cloned().unwrap_or_else(|| json!([]))).await {
        Ok(envelope) => Json(make_chat_completion_response(&state, envelope)).into_response(),
        Err(failure) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": {
                    "message": failure.message,
                    "type": "patchhive_gateway_error",
                },
                "patchhive": {
                    "attempts": failure.attempts,
                }
            })),
        )
            .into_response(),
    }
}

async fn responses_api(State(state): State<AppState>, Json(body): Json<Value>) -> impl IntoResponse {
    if body
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "message": "Streaming is not supported yet by the Rust gateway.",
                    "type": "patchhive_gateway_error",
                }
            })),
        )
            .into_response();
    }

    let messages = json!(response_input_to_messages(body.get("input").unwrap_or(&Value::Null)));
    match complete_with_fallback(&state, &body, messages).await {
        Ok(envelope) => Json(make_responses_api_response(&state, envelope)).into_response(),
        Err(failure) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": {
                    "message": failure.message,
                    "type": "patchhive_gateway_error",
                },
                "patchhive": {
                    "attempts": failure.attempts,
                }
            })),
        )
            .into_response(),
    }
}

async fn complete_with_fallback(
    state: &AppState,
    body: &Value,
    messages: Value,
) -> std::result::Result<CompletionEnvelope, CompletionFailure> {
    let requested_model = body
        .get("model")
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    let timeout_ms = body
        .get("patchhive_timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    let request_id = next_request_id(state, "ph_req");
    let product = body
        .get("patchhive_product")
        .cloned()
        .unwrap_or_else(|| json!("unknown"));

    let mut attempts = Vec::new();

    for provider in requested_providers(state, body) {
        let Some(adapter) = state.adapters.get(&provider) else {
            attempts.push(CompletionAttempt {
                provider,
                ok: false,
                model: requested_model.clone(),
                error_code: Some("provider_not_enabled".to_string()),
                retryable: Some(false),
                error: Some("provider not enabled".to_string()),
            });
            continue;
        };

        let params = json!({
            "model": requested_model.clone(),
            "messages": messages,
            "timeout_ms": timeout_ms,
            "metadata": {
                "request_id": request_id,
                "product": product,
            }
        });

        match adapter.complete(params).await {
            Ok(result) => {
                attempts.push(CompletionAttempt {
                    provider,
                    ok: true,
                    model: Some(result.model.clone()),
                    error_code: None,
                    retryable: None,
                    error: None,
                });
                return Ok(CompletionEnvelope { result, attempts });
            }
            Err(error) => attempts.push(CompletionAttempt {
                provider,
                ok: false,
                model: requested_model.clone(),
                error_code: Some(error.code.clone()),
                retryable: Some(error.retryable),
                error: Some(error.message.clone()),
            }),
        }
    }

    let message = if attempts.is_empty() {
        "No enabled providers were available.".to_string()
    } else {
        format!(
            "All local AI providers failed. {}",
            attempts
                .iter()
                .filter_map(|attempt| {
                    attempt
                        .error
                        .as_ref()
                        .map(|error| format!("{}: {}", attempt.provider, error))
                })
                .collect::<Vec<_>>()
                .join("; ")
        )
    };

    Err(CompletionFailure { message, attempts })
}

async fn spawn_adapter(name: &'static str) -> Result<AdapterClient> {
    let adapter_path = adapter_script_path(name)?;
    let client = AdapterClient {
        name,
        script_path: adapter_path.clone(),
        next_id: AtomicU64::new(1),
        restart_count: AtomicU64::new(0),
        last_restart_reason: Mutex::new(None),
        process: Mutex::new(spawn_initialized_process(name, &adapter_path).await?),
    };

    info!("spawned {name} adapter");
    Ok(client)
}

async fn spawn_initialized_process(name: &str, script_path: &PathBuf) -> Result<AdapterProcess> {
    let mut child = Command::new("node")
        .arg(script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {name} adapter at {}", script_path.display()))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to capture stdin for {name} adapter"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("failed to capture stdout for {name} adapter"))?;

    let mut process = AdapterProcess {
        child,
        stdin,
        stdout: BufReader::new(stdout).lines(),
    };

    initialize_adapter_process(name, &mut process).await?;
    Ok(process)
}

async fn initialize_adapter_process(name: &str, process: &mut AdapterProcess) -> Result<()> {
    let init = send_request_to_process(
        name,
        process,
        0,
        "initialize",
        json!({
            "adapter": name,
            "protocol_version": 1,
        }),
    )
    .await
    .map_err(|error| anyhow!(error.to_string()))?;

    let init: InitializeResult =
        serde_json::from_value(init).context("failed to decode initialize response")?;

    if !init.ready || init.protocol_version != 1 || init.adapter != name {
        return Err(anyhow!(
            "{name} adapter returned unexpected initialize payload: ready={}, protocol_version={}, adapter={}",
            init.ready,
            init.protocol_version,
            init.adapter,
        ));
    }

    Ok(())
}

async fn shutdown_adapter_process(process: &mut AdapterProcess) {
    match process.child.try_wait() {
        Ok(Some(_)) => return,
        Ok(None) | Err(_) => {}
    }

    let _ = process.child.start_kill();
    let _ = tokio::time::timeout(Duration::from_secs(3), process.child.wait()).await;
}

async fn send_request_to_process(
    name: &str,
    process: &mut AdapterProcess,
    request_id: u64,
    method: &str,
    params: Value,
) -> std::result::Result<Value, AdapterError> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params,
    });

    process
        .stdin
        .write_all(request.to_string().as_bytes())
        .await
        .map_err(|error| {
            AdapterError::transport(format!(
                "failed to write {method} request to {name} adapter: {error}"
            ))
        })?;
    process
        .stdin
        .write_all(b"\n")
        .await
        .map_err(|error| AdapterError::transport(format!("failed to write newline: {error}")))?;
    process
        .stdin
        .flush()
        .await
        .map_err(|error| AdapterError::transport(format!("failed to flush adapter stdin: {error}")))?;

    let line = process
        .stdout
        .next_line()
        .await
        .map_err(|error| {
            AdapterError::transport(format!(
                "failed to read {method} response from {name} adapter: {error}"
            ))
        })?
        .ok_or_else(|| AdapterError::transport(format!("{name} adapter closed stdout")))?;

    if let Some(status) = process
        .child
        .try_wait()
        .map_err(|error| AdapterError::transport(format!("failed to inspect adapter status: {error}")))?
    {
        if method != "shutdown" {
            warn!("{name} adapter exited unexpectedly with status {status}");
        }
    }

    let response: JsonRpcResponse = serde_json::from_str(&line).map_err(|error| {
        AdapterError::transport(format!(
            "{name} adapter returned invalid JSON: {line}. decode error: {error}"
        ))
    })?;

    if response.id != Some(json!(request_id)) {
        return Err(AdapterError::transport(format!(
            "{name} adapter returned mismatched id {:?} for request {}",
            response.id, request_id,
        )));
    }

    if let Some(error) = response.error {
        return Err(error);
    }

    response
        .result
        .ok_or_else(|| AdapterError::transport(format!("{name} adapter returned no result for {method}")))
}

impl AdapterClient {
    async fn health(&self) -> Result<AdapterHealth> {
        let value = self
            .call("health", json!({}))
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        let mut health: AdapterHealth =
            serde_json::from_value(value).context("failed to decode health response")?;
        health.restart_count = Some(self.restart_count.load(Ordering::SeqCst));
        health.last_restart_reason = self.last_restart_reason().await;
        Ok(health)
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        let value = self
            .call("list_models", json!({}))
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        let models: AdapterModels =
            serde_json::from_value(value).context("failed to decode models response")?;
        Ok(models.models)
    }

    async fn complete(&self, params: Value) -> std::result::Result<CompletionResult, AdapterError> {
        let value = self.call("complete", params).await?;
        serde_json::from_value(value).map_err(|error| {
            AdapterError::transport(format!("failed to decode completion response: {error}"))
        })
    }

    async fn call(&self, method: &str, params: Value) -> std::result::Result<Value, AdapterError> {
        let mut attempt = 0;
        loop {
            let mut process = self.process.lock().await;

            if let Some(status) = process
                .child
                .try_wait()
                .map_err(|error| AdapterError::transport(format!("failed to inspect adapter status: {error}")))?
            {
                self.restart_locked(
                    &mut process,
                    format!("{method} requested while adapter was exited with status {status}"),
                )
                .await?;
            }

            let request_id = self.next_id.fetch_add(1, Ordering::SeqCst);
            match send_request_to_process(self.name, &mut process, request_id, method, params.clone()).await {
                Ok(value) => return Ok(value),
                Err(error) if error.is_transport() && attempt == 0 => {
                    self.restart_locked(
                        &mut process,
                        format!("{method} transport failure: {}", error.message),
                    )
                    .await?;
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn restart_locked(
        &self,
        process: &mut AdapterProcess,
        reason: String,
    ) -> std::result::Result<(), AdapterError> {
        {
            let mut last_restart_reason = self.last_restart_reason.lock().await;
            *last_restart_reason = Some(reason.clone());
        }

        warn!("restarting {} adapter: {}", self.name, reason);
        shutdown_adapter_process(process).await;

        let restarted_process = spawn_initialized_process(self.name, &self.script_path)
            .await
            .map_err(|error| {
                AdapterError::transport(format!("failed to restart {} adapter: {error}", self.name))
            })?;

        *process = restarted_process;
        self.restart_count.fetch_add(1, Ordering::SeqCst);
        info!("restarted {} adapter", self.name);
        Ok(())
    }

    async fn last_restart_reason(&self) -> Option<String> {
        self.last_restart_reason.lock().await.clone()
    }
}

fn adapter_script_path(name: &str) -> Result<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../adapters")
        .join(name)
        .join("index.js");
    path.canonicalize()
        .with_context(|| format!("failed to resolve adapter path {}", path.display()))
}

fn env_bool(key: &str, fallback: bool) -> bool {
    match std::env::var(key) {
        Ok(value) => matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => fallback,
    }
}

fn resolved_provider_order(adapters: &HashMap<String, Arc<AdapterClient>>) -> Vec<String> {
    let env_value = std::env::var("PATCHHIVE_AI_PROVIDER_ORDER").unwrap_or_default();
    let mut order = if env_value.trim().is_empty() {
        DEFAULT_PROVIDER_ORDER
            .iter()
            .map(|provider| provider.to_string())
            .collect::<Vec<_>>()
    } else {
        env_value
            .split(',')
            .map(|provider| provider.trim().to_ascii_lowercase())
            .filter(|provider| !provider.is_empty())
            .collect::<Vec<_>>()
    };

    order.retain(|provider| adapters.contains_key(provider));

    for provider in adapters.keys() {
        if !order.contains(provider) {
            order.push(provider.clone());
        }
    }

    order
}

fn ordered_adapter_names(state: &AppState) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();

    for provider in &state.provider_order {
        if state.adapters.contains_key(provider) && seen.insert(provider.clone()) {
            ordered.push(provider.clone());
        }
    }

    for provider in state.adapters.keys() {
        if seen.insert(provider.clone()) {
            ordered.push(provider.clone());
        }
    }

    ordered
}

fn requested_providers(state: &AppState, body: &Value) -> Vec<String> {
    if let Some(provider) = body
        .get("patchhive_provider")
        .or_else(|| body.get("provider"))
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
    {
        return vec![provider];
    }

    ordered_adapter_names(state)
}

fn response_input_to_messages(input: &Value) -> Vec<Value> {
    match input {
        Value::String(text) => vec![json!({
            "role": "user",
            "content": text,
        })],
        Value::Array(items) => items
            .iter()
            .flat_map(response_input_item_to_messages)
            .collect(),
        Value::Null => Vec::new(),
        other => vec![json!({
            "role": "user",
            "content": other,
        })],
    }
}

fn response_input_item_to_messages(item: &Value) -> Vec<Value> {
    match item {
        Value::String(text) => vec![json!({
            "role": "user",
            "content": text,
        })],
        Value::Object(map) if map.get("type").and_then(Value::as_str) == Some("message") => vec![json!({
            "role": map.get("role").cloned().unwrap_or_else(|| json!("user")),
            "content": map.get("content").cloned().unwrap_or_else(|| json!("")),
        })],
        other => vec![json!({
            "role": "user",
            "content": other,
        })],
    }
}

fn make_chat_completion_response(state: &AppState, envelope: CompletionEnvelope) -> Value {
    let created = unix_timestamp();
    let CompletionEnvelope { result, attempts } = envelope;

    json!({
        "id": next_request_id(state, "chatcmpl"),
        "object": "chat.completion",
        "created": created,
        "model": result.model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": result.text,
            },
            "finish_reason": "stop",
        }],
        "usage": result.usage.as_ref().map(openai_usage),
        "patchhive": {
            "provider": result.provider,
            "attempts": attempts,
        }
    })
}

fn make_responses_api_response(state: &AppState, envelope: CompletionEnvelope) -> Value {
    let created = unix_timestamp();
    let CompletionEnvelope { result, attempts } = envelope;

    json!({
        "id": next_request_id(state, "resp"),
        "object": "response",
        "created_at": created,
        "model": result.model,
        "output": [{
            "id": next_request_id(state, "msg"),
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "output_text",
                "text": result.text,
                "annotations": [],
            }],
        }],
        "output_text": result.text,
        "usage": result.usage.as_ref().map(response_usage),
        "patchhive": {
            "provider": result.provider,
            "attempts": attempts,
        }
    })
}

fn openai_usage(usage: &CompletionUsage) -> Value {
    let prompt_tokens = usage.input_tokens + usage.cached_input_tokens;
    let completion_tokens = usage.output_tokens;
    json!({
        "prompt_tokens": prompt_tokens,
        "completion_tokens": completion_tokens,
        "total_tokens": prompt_tokens + completion_tokens,
    })
}

fn response_usage(usage: &CompletionUsage) -> Value {
    let input_tokens = usage.input_tokens + usage.cached_input_tokens;
    let output_tokens = usage.output_tokens;
    json!({
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "total_tokens": input_tokens + output_tokens,
    })
}

fn next_request_id(state: &AppState, prefix: &str) -> String {
    format!(
        "{}_{}",
        prefix,
        state.response_counter.fetch_add(1, Ordering::SeqCst)
    )
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
