use axum::{
    extract::{Path, State},
    Json,
    routing::{delete, get},
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;

use crate::agents::{get_cooldowns, clear_cooldown};
use crate::db::{get_conn, set_setting, get_lifetime_cost};
use crate::state::{AgentConfig, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config",                   get(get_config).post(save_config))
        .route("/ai-local/status",          get(get_ai_local_status))
        .route("/models/:provider",         get(list_models))
        .route("/agents",                   get(list_agents).post(set_team))
        .route("/agents/:id",               delete(remove_agent))
        .route("/presets",                  get(list_presets).post(save_preset))
        .route("/presets/:name",            delete(delete_preset))
        .route("/repo-lists",               get(get_repo_lists).post(add_repo))
        .route("/repo-lists/*repo",         delete(remove_repo))
        .route("/cooldowns",                get(list_cooldowns))
        .route("/cooldowns/:provider",      delete(clear_provider_cooldown))
        .route("/watch-mode",               get(get_watch_mode).post(set_watch_mode))
        .route("/stats/lifetime-cost",      get(lifetime_cost))
}

const PROVIDER_MODELS: &[(&str, &[&str])] = &[
    ("anthropic", &["claude-opus-4-6","claude-sonnet-4-6","claude-haiku-4-5","claude-sonnet-4-20250514"]),
    ("openai",    &[
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
    ]),
    ("gemini",    &["gemini-2.0-flash","gemini-2.0-flash-lite","gemini-1.5-pro","gemini-2.5-pro"]),
    ("groq",      &["llama-3.3-70b-versatile","llama-3.1-8b-instant","mixtral-8x7b-32768"]),
    ("ollama",    &["llama3.2","codellama","deepseek-coder","qwen2.5-coder"]),
];

const ROLES: &[(&str, &str, &str, &str, &str)] = &[
    ("scout",      "Scout",      "◎", "#4a9af0", "Hunts repos & judges issue quality"),
    ("judge",      "Judge",      "⚖", "#e0a030", "Targets relevant files for the kill"),
    ("reaper",     "Reaper",     "⚔", "#c41e3a", "Forges the killing patch"),
    ("smith",      "Smith",      "⬢", "#7b2d8b", "Refines & improves patches"),
    ("gatekeeper", "Gatekeeper", "🔒","#2a8a4a", "Validates & opens PRs"),
];

fn env(k: &str) -> String { std::env::var(k).unwrap_or_default() }

fn normalize_repo_list_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "allowlist" => Some("allowlist"),
        "denylist" | "blocklist" => Some("denylist"),
        "opt_out" | "opt-out" | "optout" => Some("opt_out"),
        _ => None,
    }
}

async fn get_config(State(state): State<AppState>) -> Json<Value> {
    let providers: Value = PROVIDER_MODELS.iter().map(|(p, models)| {
        (p.to_string(), Value::Array(models.iter().map(|m| json!(m)).collect::<Vec<_>>()))
    }).collect::<serde_json::Map<_, _>>().into();

    let roles: Value = ROLES.iter().map(|(id, label, icon, color, desc)| {
        (id.to_string(), json!({"label": label, "icon": icon, "color": color, "desc": desc}))
    }).collect::<serde_json::Map<_, _>>().into();

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
    #[serde(rename = "BOT_GITHUB_TOKEN")]  bot_token:  Option<String>,
    #[serde(rename = "BOT_GITHUB_USER")]   bot_user:   Option<String>,
    #[serde(rename = "BOT_GITHUB_EMAIL")]  bot_email:  Option<String>,
    #[serde(rename = "PROVIDER_API_KEY")]  api_key:    Option<String>,
    #[serde(rename = "PATCHHIVE_AI_URL")]  patchhive_ai_url: Option<String>,
    #[serde(rename = "OLLAMA_BASE_URL")]   ollama_url: Option<String>,
    #[serde(rename = "WEBHOOK_SECRET")]    webhook_secret: Option<String>,
    #[serde(rename = "COST_BUDGET_USD")]   cost_budget: Option<String>,
    #[serde(rename = "MIN_REVIEW_CONFIDENCE")] min_conf: Option<String>,
}

async fn save_config(Json(body): Json<ConfigSave>) -> Json<Value> {
    let pairs = [
        ("BOT_GITHUB_TOKEN",      body.bot_token),
        ("BOT_GITHUB_USER",       body.bot_user),
        ("BOT_GITHUB_EMAIL",      body.bot_email),
        ("PROVIDER_API_KEY",      body.api_key),
        ("PATCHHIVE_AI_URL",      body.patchhive_ai_url),
        ("OLLAMA_BASE_URL",       body.ollama_url),
        ("WEBHOOK_SECRET",        body.webhook_secret),
        ("COST_BUDGET_USD",       body.cost_budget),
        ("MIN_REVIEW_CONFIDENCE", body.min_conf),
    ];
    for (key, val) in pairs {
        if let Some(v) = val {
            let is_masked_placeholder = matches!(key, "BOT_GITHUB_TOKEN" | "PROVIDER_API_KEY" | "WEBHOOK_SECRET")
                && (v == "(set)" || v.starts_with('*'));
            if !v.is_empty() && !is_masked_placeholder {
                std::env::set_var(key, &v);
            }
        }
    }
    Json(json!({"saved": true}))
}

async fn list_models(State(state): State<AppState>, Path(provider): Path<String>) -> Json<Value> {
    let fallback_models = PROVIDER_MODELS.iter().find(|(p, _)| *p == provider.as_str())
        .map(|(_, m)| m.to_vec())
        .unwrap_or_default();

    if provider == "openai" && crate::ai_local::configured_url().is_some() {
        return match crate::ai_local::fetch_models(&state.http).await {
            Ok(models) if !models.is_empty() => Json(json!({
                "models": models,
                "source": "patchhive-ai-local",
            })),
            Ok(_) => Json(json!({
                "models": fallback_models,
                "source": "static_fallback",
                "error": "PatchHive AI gateway returned no models",
            })),
            Err(error) => Json(json!({
                "models": fallback_models,
                "source": "static_fallback",
                "error": error.to_string(),
            })),
        };
    }

    Json(json!({"models": fallback_models, "source": "static"}))
}

async fn list_agents(State(state): State<AppState>) -> Json<Value> {
    let agents: Vec<_> = state.agents.read().await.values().cloned().collect();
    let cooldowns = get_cooldowns().await;
    Json(json!({"agents": agents, "cooldowns": cooldowns}))
}

#[derive(Deserialize)]
struct TeamBody { agents: Vec<AgentConfig> }

async fn set_team(State(state): State<AppState>, Json(body): Json<TeamBody>) -> Json<Value> {
    let mut map = state.agents.write().await;
    map.clear();
    for mut a in body.agents {
        if a.id.is_empty() { a.id = Uuid::new_v4().to_string()[..8].to_string(); }
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
    let Ok(conn) = get_conn() else { return Json(json!({"presets":[]})) };
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
struct PresetSave { name: String, agents: Vec<Value> }

async fn save_preset(Json(body): Json<PresetSave>) -> Json<Value> {
    let Ok(conn) = get_conn() else { return Json(json!({"saved":false})) };
    let _ = conn.execute(
        "INSERT OR REPLACE INTO team_presets(name, agents_json, created_at) VALUES(?1,?2,?3)",
        rusqlite::params![body.name, serde_json::to_string(&body.agents).unwrap_or_default(), Utc::now().to_rfc3339()],
    );
    Json(json!({"saved": true}))
}

async fn delete_preset(Path(name): Path<String>) -> Json<Value> {
    let Ok(conn) = get_conn() else { return Json(json!({"ok":false})) };
    let _ = conn.execute("DELETE FROM team_presets WHERE name=?1", [&name]);
    Json(json!({"ok": true}))
}

async fn get_repo_lists() -> Json<Value> {
    let Ok(conn) = get_conn() else { return Json(json!({"repos":[]})) };
    let rows: Vec<Value> = conn.prepare("SELECT repo, list_type, added_at FROM repo_lists").ok()
        .and_then(|mut s| {
            let mapped = s.query_map([], |r| {
                let list_type = r.get::<_, String>(1)?;
                Ok(json!({
                    "repo": r.get::<_, String>(0)?,
                    "list_type": normalize_repo_list_type(&list_type).unwrap_or("denylist"),
                    "added_at": r.get::<_, String>(2)?,
                }))
            }).ok()?;
            Some(mapped.flatten().collect())
        })
        .unwrap_or_default();
    Json(json!({"repos": rows}))
}

#[derive(Deserialize)]
struct RepoListUpdate { repo: String, list_type: String }

async fn add_repo(Json(body): Json<RepoListUpdate>) -> Json<Value> {
    let Ok(conn) = get_conn() else { return Json(json!({"ok":false})) };
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
    let Ok(conn) = get_conn() else { return Json(json!({"ok":false})) };
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
struct WatchModeBody { enabled: bool }

async fn set_watch_mode(State(state): State<AppState>, Json(body): Json<WatchModeBody>) -> Json<Value> {
    state.watch_mode.store(body.enabled, std::sync::atomic::Ordering::SeqCst);
    let _ = set_setting("watch_mode", if body.enabled { "true" } else { "false" });
    Json(json!({"watch_mode": body.enabled}))
}

async fn lifetime_cost() -> Json<Value> {
    Json(json!({"lifetime_cost_usd": get_lifetime_cost()}))
}
