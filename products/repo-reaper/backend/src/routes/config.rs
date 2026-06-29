use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, fs, path::Path as StdPath, time::Duration};
use uuid::Uuid;

use crate::agents::{clear_cooldown, get_cooldowns};
use crate::db::{get_conn, get_lifetime_cost, set_setting};
use crate::state::{AgentConfig, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config", get(get_config).post(save_config))
        .route("/ai-local/status", get(get_ai_local_status))
        .route("/models/:provider", get(list_models).post(refresh_models))
        .route("/agents", get(list_agents).post(set_team))
        .route("/agents/:id", delete(remove_agent))
        .route("/presets", get(list_presets).post(save_preset))
        .route("/presets/:name", delete(delete_preset))
        .route("/repo-lists", get(get_repo_lists).post(add_repo))
        .route("/repo-lists/*repo", delete(remove_repo))
        .route("/cooldowns", get(list_cooldowns))
        .route("/cooldowns/:provider", delete(clear_provider_cooldown))
        .route("/watch-mode", get(get_watch_mode).post(set_watch_mode))
        .route("/stats/lifetime-cost", get(lifetime_cost))
}

const PROVIDER_MODELS: &[(&str, &[&str])] = &[
    (
        "anthropic",
        &[
            "claude-opus-4-6",
            "claude-sonnet-4-6",
            "claude-haiku-4-5",
            "claude-sonnet-4-20250514",
        ],
    ),
    (
        "openai",
        &[
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.4-nano",
            "gpt-5.3-codex",
            "gpt-5.2-codex",
            "gpt-5.1",
            "gpt-5-mini",
            "gpt-5-nano",
            "gpt-5.1-codex",
            "gpt-5.1-codex-mini",
            "gpt-5.1-codex-max",
            "gpt-5-codex",
            "gpt-5",
            "gpt-4.1",
            "gpt-4.1-mini",
            "gpt-4.1-nano",
            "o3",
            "o4-mini",
            "o3-mini",
        ],
    ),
    (
        "gemini",
        &[
            "gemini-2.0-flash",
            "gemini-2.0-flash-lite",
            "gemini-1.5-pro",
            "gemini-2.5-pro",
        ],
    ),
    (
        "groq",
        &[
            "llama-3.3-70b-versatile",
            "llama-3.1-8b-instant",
            "mixtral-8x7b-32768",
        ],
    ),
    ("custom", &["gpt-4.1-mini", "qwen2.5-coder", "llama3.2"]),
    (
        "ollama",
        &["llama3.2", "codellama", "deepseek-coder", "qwen2.5-coder"],
    ),
];

const ROLES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "scout",
        "Scout",
        "◎",
        "#4a9af0",
        "Hunts repos & judges issue quality",
    ),
    (
        "judge",
        "Judge",
        "⚖",
        "#e0a030",
        "Targets relevant files for the kill",
    ),
    (
        "reaper",
        "Reaper",
        "⚔",
        "#c41e3a",
        "Forges the killing patch",
    ),
    (
        "smith",
        "Smith",
        "⬢",
        "#7b2d8b",
        "Refines & improves patches",
    ),
    (
        "gatekeeper",
        "Gatekeeper",
        "🔒",
        "#2a8a4a",
        "Validates & opens PRs",
    ),
];

fn env(k: &str) -> String {
    env_from_file(StdPath::new(".env"), k)
        .or_else(|| std::env::var(k).ok())
        .unwrap_or_default()
}

fn env_from_file(path: &StdPath, key: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            return None;
        }
        let (line_key, value) = trimmed.split_once('=')?;
        if line_key.trim() == key {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

fn persist_env_updates(path: &StdPath, updates: &[(String, String)]) -> std::io::Result<()> {
    let update_keys: HashSet<&str> = updates.iter().map(|(key, _)| key.as_str()).collect();
    let existing = fs::read_to_string(path).unwrap_or_default();
    let mut retained = existing
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !update_keys
                .iter()
                .any(|key| trimmed.starts_with(&format!("{key}=")))
        })
        .map(str::to_string)
        .collect::<Vec<_>>();

    for (key, value) in updates {
        retained.push(format!("{key}={value}"));
    }

    let mut content = retained.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    fs::write(path, content)
}

fn normalize_repo_list_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "allowlist" => Some("allowlist"),
        "denylist" | "blocklist" => Some("denylist"),
        "opt_out" | "opt-out" | "optout" => Some("opt_out"),
        _ => None,
    }
}

async fn get_config(State(state): State<AppState>) -> Json<Value> {
    let providers: Value = PROVIDER_MODELS
        .iter()
        .map(|(p, models)| {
            (
                p.to_string(),
                Value::Array(models.iter().map(|m| json!(m)).collect::<Vec<_>>()),
            )
        })
        .collect::<serde_json::Map<_, _>>()
        .into();

    let roles: Value = ROLES
        .iter()
        .map(|(id, label, icon, color, desc)| {
            (
                id.to_string(),
                json!({"label": label, "icon": icon, "color": color, "desc": desc}),
            )
        })
        .collect::<serde_json::Map<_, _>>()
        .into();

    Json(json!({
        "BOT_GITHUB_TOKEN":      "",
        "BOT_GITHUB_TOKEN_SET":  !env("BOT_GITHUB_TOKEN").is_empty(),
        "BOT_GITHUB_USER":       env("BOT_GITHUB_USER"),
        "BOT_GITHUB_EMAIL":      env("BOT_GITHUB_EMAIL"),
        "PROVIDER_API_KEY":      "",
        "PROVIDER_API_KEY_SET":  !env("PROVIDER_API_KEY").is_empty(),
        "PATCHHIVE_AI_URL":      env("PATCHHIVE_AI_URL"),
        "AI_LOCAL_STATUS":       crate::ai_local::fetch_status(&state.http).await,
        "OLLAMA_BASE_URL":       env("OLLAMA_BASE_URL"),
        "WEBHOOK_SECRET":        "",
        "WEBHOOK_SECRET_SET":    !env("WEBHOOK_SECRET").is_empty(),
        "COST_BUDGET_USD":       env("COST_BUDGET_USD"),
        "MIN_REVIEW_CONFIDENCE": env("MIN_REVIEW_CONFIDENCE"),
        "providers": providers,
        "roles": roles,
    }))
}

async fn get_ai_local_status(State(state): State<AppState>) -> Json<Value> {
    Json(crate::ai_local::fetch_status(&state.http).await)
}

#[derive(Deserialize)]
struct ConfigSave {
    #[serde(rename = "BOT_GITHUB_TOKEN")]
    bot_token: Option<String>,
    #[serde(rename = "BOT_GITHUB_USER")]
    bot_user: Option<String>,
    #[serde(rename = "BOT_GITHUB_EMAIL")]
    bot_email: Option<String>,
    #[serde(rename = "PROVIDER_API_KEY")]
    api_key: Option<String>,
    #[serde(rename = "PATCHHIVE_AI_URL")]
    patchhive_ai_url: Option<String>,
    #[serde(rename = "OLLAMA_BASE_URL")]
    ollama_url: Option<String>,
    #[serde(rename = "WEBHOOK_SECRET")]
    webhook_secret: Option<String>,
    #[serde(rename = "COST_BUDGET_USD")]
    cost_budget: Option<String>,
    #[serde(rename = "MIN_REVIEW_CONFIDENCE")]
    min_conf: Option<String>,
}

async fn save_config(Json(body): Json<ConfigSave>) -> Json<Value> {
    let pairs = [
        ("BOT_GITHUB_TOKEN", body.bot_token),
        ("BOT_GITHUB_USER", body.bot_user),
        ("BOT_GITHUB_EMAIL", body.bot_email),
        ("PROVIDER_API_KEY", body.api_key),
        ("PATCHHIVE_AI_URL", body.patchhive_ai_url),
        ("OLLAMA_BASE_URL", body.ollama_url),
        ("WEBHOOK_SECRET", body.webhook_secret),
        ("COST_BUDGET_USD", body.cost_budget),
        ("MIN_REVIEW_CONFIDENCE", body.min_conf),
    ];
    let mut updates = Vec::new();
    for (key, val) in pairs {
        if let Some(v) = val {
            let is_masked_placeholder = matches!(
                key,
                "BOT_GITHUB_TOKEN" | "PROVIDER_API_KEY" | "WEBHOOK_SECRET"
            ) && (v == "(set)" || v.starts_with('*'));
            if !v.is_empty() && !is_masked_placeholder {
                updates.push((key.to_string(), v));
            }
        }
    }

    let saved = persist_env_updates(StdPath::new(".env"), &updates).is_ok();
    Json(json!({"saved": saved, "restart_required": saved}))
}

#[derive(Deserialize)]
struct ModelDiscoveryBody {
    api_key: Option<String>,
    base_url: Option<String>,
}

async fn list_models(State(state): State<AppState>, Path(provider): Path<String>) -> Json<Value> {
    model_discovery_response(&state.http, &provider, None).await
}

async fn refresh_models(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Json(body): Json<ModelDiscoveryBody>,
) -> Json<Value> {
    model_discovery_response(&state.http, &provider, Some(body)).await
}

async fn model_discovery_response(
    http: &reqwest::Client,
    provider: &str,
    body: Option<ModelDiscoveryBody>,
) -> Json<Value> {
    let fallback_models = static_provider_models(provider);
    if fallback_models.is_empty() {
        return Json(json!({
            "models": [],
            "source": "unsupported",
            "error": format!("Unknown provider: {provider}"),
        }));
    }

    match discover_provider_models(http, provider, body).await {
        Ok((models, source)) if !models.is_empty() => Json(json!({
            "models": models,
            "source": source,
        })),
        Ok((_, source)) => Json(json!({
            "models": fallback_models,
            "source": "static_fallback",
            "error": format!("{source} returned no models"),
        })),
        Err(error) => Json(json!({
            "models": fallback_models,
            "source": "static_fallback",
            "error": error.to_string(),
        })),
    }
}

async fn discover_provider_models(
    http: &reqwest::Client,
    provider: &str,
    body: Option<ModelDiscoveryBody>,
) -> anyhow::Result<(Vec<String>, &'static str)> {
    match provider {
        "openai" => discover_openai_models(http, body).await,
        "anthropic" => discover_anthropic_models(http, body).await,
        "gemini" => discover_gemini_models(http, body).await,
        "groq" => discover_groq_models(http, body).await,
        "custom" => discover_custom_models(http, body).await,
        "ollama" => discover_ollama_models(http, body).await,
        _ => anyhow::bail!("Unknown provider: {provider}"),
    }
}

async fn discover_openai_models(
    http: &reqwest::Client,
    body: Option<ModelDiscoveryBody>,
) -> anyhow::Result<(Vec<String>, &'static str)> {
    let supplied_key = clean_optional(body.as_ref().and_then(|b| b.api_key.as_deref()));
    let supplied_base = clean_optional(body.as_ref().and_then(|b| b.base_url.as_deref()));

    if supplied_key.is_none()
        && supplied_base.is_none()
        && crate::ai_local::configured_url().is_some()
    {
        return Ok((
            crate::ai_local::fetch_models(http).await?,
            "patchhive-ai-local",
        ));
    }

    let base = supplied_base
        .or_else(|| clean_env("OPENAI_BASE_URL"))
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let key = supplied_key.or_else(|| provider_api_key("openai"));
    Ok((
        fetch_openai_compatible_models(http, &base, key.as_deref()).await?,
        "provider-api",
    ))
}

async fn discover_groq_models(
    http: &reqwest::Client,
    body: Option<ModelDiscoveryBody>,
) -> anyhow::Result<(Vec<String>, &'static str)> {
    let base = clean_optional(body.as_ref().and_then(|b| b.base_url.as_deref()))
        .or_else(|| clean_env("GROQ_BASE_URL"))
        .unwrap_or_else(|| "https://api.groq.com/openai/v1".to_string());
    let key = clean_optional(body.as_ref().and_then(|b| b.api_key.as_deref()))
        .or_else(|| provider_api_key("groq"));
    Ok((
        fetch_openai_compatible_models(http, &base, key.as_deref()).await?,
        "provider-api",
    ))
}

async fn discover_custom_models(
    http: &reqwest::Client,
    body: Option<ModelDiscoveryBody>,
) -> anyhow::Result<(Vec<String>, &'static str)> {
    let base = clean_optional(body.as_ref().and_then(|b| b.base_url.as_deref()))
        .or_else(|| clean_env("CUSTOM_AI_BASE_URL"))
        .ok_or_else(|| {
            anyhow::anyhow!("No custom OpenAI-compatible base URL available for model discovery")
        })?;
    let key = clean_optional(body.as_ref().and_then(|b| b.api_key.as_deref()))
        .or_else(|| provider_api_key("custom"));
    Ok((
        fetch_openai_compatible_models(http, &base, key.as_deref()).await?,
        "provider-api",
    ))
}

async fn discover_anthropic_models(
    http: &reqwest::Client,
    body: Option<ModelDiscoveryBody>,
) -> anyhow::Result<(Vec<String>, &'static str)> {
    let key = clean_optional(body.as_ref().and_then(|b| b.api_key.as_deref()))
        .or_else(|| provider_api_key("anthropic"))
        .ok_or_else(|| anyhow::anyhow!("No Anthropic API key available for model discovery"))?;

    #[derive(Deserialize)]
    struct AnthropicModels {
        data: Vec<ProviderModel>,
    }

    let resp = http
        .get("https://api.anthropic.com/v1/models")
        .timeout(Duration::from_secs(8))
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic model discovery returned {status}: {body}");
    }

    let models = resp
        .json::<AnthropicModels>()
        .await?
        .data
        .into_iter()
        .map(|model| model.id)
        .collect();
    Ok((clean_model_ids(models), "provider-api"))
}

async fn discover_gemini_models(
    http: &reqwest::Client,
    body: Option<ModelDiscoveryBody>,
) -> anyhow::Result<(Vec<String>, &'static str)> {
    let key = clean_optional(body.as_ref().and_then(|b| b.api_key.as_deref()))
        .or_else(|| provider_api_key("gemini"))
        .ok_or_else(|| anyhow::anyhow!("No Gemini API key available for model discovery"))?;

    #[derive(Deserialize)]
    struct GeminiModels {
        models: Vec<GeminiModel>,
    }

    #[derive(Deserialize)]
    struct GeminiModel {
        name: String,
    }

    let resp = http
        .get("https://generativelanguage.googleapis.com/v1beta/models")
        .timeout(Duration::from_secs(8))
        .header("x-goog-api-key", key)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Gemini model discovery returned {status}: {body}");
    }

    let models = resp
        .json::<GeminiModels>()
        .await?
        .models
        .into_iter()
        .map(|model| {
            model
                .name
                .strip_prefix("models/")
                .unwrap_or(&model.name)
                .to_string()
        })
        .collect();
    Ok((clean_model_ids(models), "provider-api"))
}

async fn discover_ollama_models(
    http: &reqwest::Client,
    body: Option<ModelDiscoveryBody>,
) -> anyhow::Result<(Vec<String>, &'static str)> {
    #[derive(Deserialize)]
    struct OllamaTags {
        models: Vec<OllamaModel>,
    }

    #[derive(Deserialize)]
    struct OllamaModel {
        name: String,
    }

    let base = clean_optional(body.as_ref().and_then(|b| b.base_url.as_deref()))
        .or_else(|| clean_env("OLLAMA_BASE_URL"))
        .unwrap_or_else(|| "http://localhost:11434".to_string());
    let resp = http
        .get(format!("{}/api/tags", base.trim_end_matches('/')))
        .timeout(Duration::from_secs(8))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Ollama model discovery returned {status}: {body}");
    }

    let models = resp
        .json::<OllamaTags>()
        .await?
        .models
        .into_iter()
        .map(|model| model.name)
        .collect();
    Ok((clean_model_ids(models), "ollama"))
}

#[derive(Deserialize)]
struct ProviderModel {
    id: String,
}

#[derive(Deserialize)]
struct OpenAiModels {
    data: Vec<ProviderModel>,
}

async fn fetch_openai_compatible_models(
    http: &reqwest::Client,
    base: &str,
    api_key: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    let mut request = http
        .get(format!("{}/models", base.trim_end_matches('/')))
        .timeout(Duration::from_secs(8));
    request = match api_key {
        Some(key) => request.bearer_auth(key),
        None if crate::ai_local::is_local_openai_base(base) => {
            request.bearer_auth("patchhive-local")
        }
        None => anyhow::bail!("No OpenAI-compatible API key available for model discovery"),
    };

    let resp = request.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI-compatible model discovery returned {status}: {body}");
    }

    let models = resp
        .json::<OpenAiModels>()
        .await?
        .data
        .into_iter()
        .map(|model| model.id)
        .collect();
    Ok(clean_model_ids(models))
}

fn static_provider_models(provider: &str) -> Vec<String> {
    PROVIDER_MODELS
        .iter()
        .find(|(p, _)| *p == provider)
        .map(|(_, models)| models.iter().map(|model| model.to_string()).collect())
        .unwrap_or_default()
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn clean_env(key: &str) -> Option<String> {
    clean_optional(Some(env(key).as_str()))
}

fn provider_api_key(provider: &str) -> Option<String> {
    let provider_specific = match provider {
        "anthropic" => clean_env("ANTHROPIC_API_KEY"),
        "openai" => clean_env("OPENAI_API_KEY"),
        "gemini" => clean_env("GEMINI_API_KEY").or_else(|| clean_env("GOOGLE_API_KEY")),
        "groq" => clean_env("GROQ_API_KEY"),
        "custom" => clean_env("CUSTOM_AI_API_KEY").or_else(|| clean_env("OPENAI_API_KEY")),
        _ => None,
    };

    provider_specific.or_else(|| clean_env("PROVIDER_API_KEY"))
}

fn clean_model_ids(models: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    models
        .into_iter()
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
        .filter(|model| seen.insert(model.clone()))
        .collect()
}

async fn list_agents(State(state): State<AppState>) -> Json<Value> {
    let agents: Vec<_> = state.agents.read().await.values().cloned().collect();
    let cooldowns = get_cooldowns().await;
    Json(json!({"agents": agents, "cooldowns": cooldowns}))
}

#[derive(Deserialize)]
struct TeamBody {
    agents: Vec<AgentConfig>,
}

async fn set_team(State(state): State<AppState>, Json(body): Json<TeamBody>) -> Json<Value> {
    let mut map = state.agents.write().await;
    map.clear();
    for mut a in body.agents {
        if a.id.is_empty() {
            a.id = Uuid::new_v4().to_string()[..8].to_string();
        }
        a.api_key = clean_optional(a.api_key.as_deref());
        a.base_url = clean_optional(a.base_url.as_deref());
        a.bot_token = clean_optional(a.bot_token.as_deref());
        a.bot_user = clean_optional(a.bot_user.as_deref());
        a.status = "idle".into();
        a.current_task = String::new();
        map.insert(a.id.clone(), a);
    }
    Json(json!({"agents": map.values().cloned().collect::<Vec<_>>()}))
}

async fn remove_agent(State(state): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    state.agents.write().await.remove(&id);
    Json(json!({"ok": true}))
}

async fn list_presets() -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"presets":[]}));
    };
    let rows: Vec<Value> = conn.prepare("SELECT name, agents_json, created_at FROM team_presets ORDER BY created_at DESC").ok()
        .and_then(|mut s| {
            let mapped = s.query_map([], |r| {
            Ok(json!({"name": r.get::<_,String>(0)?, "agents": serde_json::from_str::<Value>(&r.get::<_,String>(1)?).unwrap_or_default(), "created_at": r.get::<_,String>(2)?}))
            }).ok()?;
            Some(mapped.flatten().collect())
        })
        .unwrap_or_default();
    Json(json!({"presets": rows}))
}

#[derive(Deserialize)]
struct PresetSave {
    name: String,
    agents: Vec<Value>,
}

async fn save_preset(Json(body): Json<PresetSave>) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"saved":false}));
    };
    let _ = conn.execute(
        "INSERT OR REPLACE INTO team_presets(name, agents_json, created_at) VALUES(?1,?2,?3)",
        rusqlite::params![
            body.name,
            serde_json::to_string(&body.agents).unwrap_or_default(),
            Utc::now().to_rfc3339()
        ],
    );
    Json(json!({"saved": true}))
}

async fn delete_preset(Path(name): Path<String>) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"ok":false}));
    };
    let _ = conn.execute("DELETE FROM team_presets WHERE name=?1", [&name]);
    Json(json!({"ok": true}))
}

async fn get_repo_lists() -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"repos":[]}));
    };
    let rows: Vec<Value> = conn
        .prepare("SELECT repo, list_type, added_at FROM repo_lists")
        .ok()
        .and_then(|mut s| {
            let mapped = s
                .query_map([], |r| {
                    let list_type = r.get::<_, String>(1)?;
                    Ok(json!({
                        "repo": r.get::<_, String>(0)?,
                        "list_type": normalize_repo_list_type(&list_type).unwrap_or("denylist"),
                        "added_at": r.get::<_, String>(2)?,
                    }))
                })
                .ok()?;
            Some(mapped.flatten().collect())
        })
        .unwrap_or_default();
    Json(json!({"repos": rows}))
}

#[derive(Deserialize)]
struct RepoListUpdate {
    repo: String,
    list_type: String,
}

async fn add_repo(Json(body): Json<RepoListUpdate>) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"ok":false}));
    };
    let Some(list_type) = normalize_repo_list_type(&body.list_type) else {
        return Json(json!({"ok": false, "error": "invalid list_type"}));
    };
    let _ = conn.execute(
        "INSERT OR REPLACE INTO repo_lists(repo, list_type, added_at) VALUES(?1,?2,?3)",
        rusqlite::params![body.repo, list_type, Utc::now().to_rfc3339()],
    );
    Json(json!({"ok": true}))
}

async fn remove_repo(Path(repo): Path<String>) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"ok":false}));
    };
    let _ = conn.execute("DELETE FROM repo_lists WHERE repo=?1", [&repo]);
    Json(json!({"ok": true}))
}

async fn list_cooldowns() -> Json<Value> {
    Json(json!({"cooldowns": get_cooldowns().await}))
}

async fn clear_provider_cooldown(Path(provider): Path<String>) -> Json<Value> {
    clear_cooldown(&provider).await;
    Json(json!({"ok": true}))
}

async fn get_watch_mode(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"watch_mode": state.watch_mode.load(std::sync::atomic::Ordering::SeqCst)}))
}

#[derive(Deserialize)]
struct WatchModeBody {
    enabled: bool,
}

async fn set_watch_mode(
    State(state): State<AppState>,
    Json(body): Json<WatchModeBody>,
) -> Json<Value> {
    state
        .watch_mode
        .store(body.enabled, std::sync::atomic::Ordering::SeqCst);
    let _ = set_setting("watch_mode", if body.enabled { "true" } else { "false" });
    Json(json!({"watch_mode": body.enabled}))
}

async fn lifetime_cost() -> Json<Value> {
    Json(json!({"lifetime_cost_usd": get_lifetime_cost()}))
}
