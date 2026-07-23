// agents.rs — Multi-provider AI calls for all RepoReaper agent roles
// Uses direct HTTP (reqwest) for full provider control.
// yoagent is used in Praxis for the full agent loop; here we do one-shot completions.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub const DEFAULT_MAX_TOKENS: u32 = 2000;
pub const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
const PATCH_MAX_TOKENS: u32 = 8000;
const REVIEW_MAX_TOKENS: u32 = 5000;

// ── Cost table ($/1k tokens: input, output) ───────────────────────────────────
fn cost_rates(provider: &str, model: &str) -> (f64, f64) {
    match (provider, model) {
        ("anthropic", m) if m.contains("opus") => (0.015, 0.075),
        ("anthropic", m) if m.contains("sonnet") => (0.003, 0.015),
        ("anthropic", _) => (0.00025, 0.00125),
        ("openai", m) if m.contains("gpt-4o") && !m.contains("mini") => (0.0025, 0.01),
        ("openai", m) if m.contains("mini") => (0.00015, 0.0006),
        ("openai", _) => (0.0025, 0.01),
        ("gemini", _) => (0.00035, 0.00105),
        ("groq", _) => (0.00059, 0.00079),
        ("ollama", _) => (0.0, 0.0),
        _ => (0.003, 0.015),
    }
}

fn estimate_cost(prompt: &str, response: &str, provider: &str, model: &str) -> f64 {
    let (ic, oc) = cost_rates(provider, model);
    // Use char count instead of byte count — more accurate for non-ASCII content.
    (prompt.chars().count() as f64 / 4.0 / 1000.0) * ic
        + (response.chars().count() as f64 / 4.0 / 1000.0) * oc
}

fn strip_json_fence(s: &str) -> &str {
    let s = s.trim();
    let s = s.strip_prefix("```json").unwrap_or(s);
    let s = s.strip_prefix("```").unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    s.trim()
}

pub fn parse_json(text: &str) -> Result<Value> {
    let clean = strip_json_fence(text);
    serde_json::from_str(clean).map_err(|e| {
        if clean.is_empty() {
            return anyhow!("Provider returned an empty response where JSON was expected");
        }
        let preview = clean
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(240)
            .collect::<String>();
        anyhow!(
            "Provider returned non-JSON output where JSON was expected: {e}. Preview: {preview}"
        )
    })
}

fn parse_typed_json<T: DeserializeOwned>(text: &str, contract: &str) -> Result<T> {
    let clean = strip_json_fence(text);
    serde_json::from_str(clean).map_err(|error| {
        if clean.is_empty() {
            return anyhow!("Provider returned an empty response for {contract}");
        }
        let preview = clean
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(240)
            .collect::<String>();
        anyhow!("Provider response violated the {contract} contract: {error}. Preview: {preview}")
    })
}

fn deserialize_nullable_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GeneratedPatchResponse {
    pub explanation: String,
    pub files_changed: Vec<String>,
    #[serde(deserialize_with = "deserialize_nullable_string")]
    pub patch: Option<String>,
    pub confidence: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PatchRetryResponse {
    pub explanation: String,
    pub files_changed: Vec<String>,
    #[serde(deserialize_with = "deserialize_nullable_string")]
    pub patch: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SmithReviewResponse {
    pub approved: bool,
    pub confidence: i32,
    pub feedback: String,
    #[serde(deserialize_with = "deserialize_nullable_string")]
    pub improved_patch: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DryRunTopCandidate {
    pub repo: String,
    pub title: String,
    pub score: i32,
    pub why: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DryRunCandidate {
    pub repo: String,
    pub title: String,
    pub score: i32,
    pub call: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DryRunAnalysisResponse {
    pub summary: String,
    pub top_candidate: DryRunTopCandidate,
    pub success_band: String,
    pub risk: String,
    pub recommendation: String,
    pub candidates: Vec<DryRunCandidate>,
}

fn clean_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn provider_api_key(provider: &str) -> Option<String> {
    let provider_specific = match provider {
        "openai" => clean_env("OPENAI_API_KEY"),
        "anthropic" => clean_env("ANTHROPIC_API_KEY"),
        "gemini" => clean_env("GEMINI_API_KEY").or_else(|| clean_env("GOOGLE_API_KEY")),
        "groq" => clean_env("GROQ_API_KEY"),
        "openrouter" => clean_env("OPENROUTER_API_KEY"),
        "custom" => clean_env("CUSTOM_AI_API_KEY").or_else(|| clean_env("OPENAI_API_KEY")),
        _ => None,
    };

    provider_specific.or_else(|| clean_env("PROVIDER_API_KEY"))
}

fn agent_or_env_api_key(p: &AgentCallParams<'_>) -> Option<String> {
    p.api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| provider_api_key(p.provider))
}

// ── Provider cooldowns ─────────────────────────────────────────────────────────

static COOLDOWNS: Lazy<RwLock<HashMap<String, Instant>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub async fn provider_available(provider: &str) -> bool {
    let map = COOLDOWNS.read().await;
    map.get(provider)
        .map(|t| Instant::now() >= *t)
        .unwrap_or(true)
}

pub async fn set_cooldown(provider: &str, secs: u64) {
    let mut map = COOLDOWNS.write().await;
    map.insert(
        provider.to_string(),
        Instant::now() + Duration::from_secs(secs),
    );
}

pub async fn get_cooldowns() -> HashMap<String, f64> {
    let map = COOLDOWNS.read().await;
    let now = Instant::now();
    map.iter()
        .filter(|(_, t)| **t > now)
        .map(|(k, t)| (k.clone(), (*t - now).as_secs_f64()))
        .collect()
}

pub async fn clear_cooldown(provider: &str) {
    COOLDOWNS.write().await.remove(provider);
}

// ── Core LLM call ──────────────────────────────────────────────────────────────

pub struct AgentCallParams<'a> {
    pub provider: &'a str,
    pub model: &'a str,
    pub base_url: Option<&'a str>,
    pub api_key: Option<&'a str>,
    pub system: &'a str,
    pub prompt: &'a str,
    pub max_tokens: u32,
    pub reasoning_effort: &'static str,
}

pub async fn ai_call(http: &Client, p: &AgentCallParams<'_>) -> Result<(String, f64)> {
    if !provider_available(p.provider).await {
        return Err(anyhow!("Provider {} is cooling down", p.provider));
    }

    let result = match p.provider {
        "anthropic" => anthropic_call(http, p).await,
        "openai" => {
            let base = crate::ai_local::openai_base_url();
            openai_call(http, p, &base).await
        }
        "gemini" => gemini_call(http, p).await,
        "groq" => {
            let base = std::env::var("GROQ_BASE_URL")
                .unwrap_or_else(|_| "https://api.groq.com/openai/v1".into());
            openai_call(http, p, &base).await
        }
        "openrouter" => {
            let base = p
                .base_url
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .or_else(|| clean_env("OPENROUTER_BASE_URL"))
                .unwrap_or_else(|| OPENROUTER_BASE_URL.into());
            openai_call(http, p, &base).await
        }
        "custom" => {
            let base = p
                .base_url
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .or_else(|| std::env::var("CUSTOM_AI_BASE_URL").ok())
                .ok_or_else(|| anyhow!("No custom OpenAI-compatible base URL configured"))?;
            openai_call(http, p, &base).await
        }
        "ollama" => ollama_call(http, p).await,
        _ => Err(anyhow!("Unknown provider: {}", p.provider)),
    };

    if let Err(ref e) = result {
        let msg = e.to_string().to_lowercase();
        if msg.contains("rate limit") || msg.contains("429") || msg.contains("quota") {
            set_cooldown(p.provider, 90).await;
        }
    }

    let (text, provider_cost) = result?;
    let cost = if p.provider == "openrouter" {
        provider_cost
    } else {
        estimate_cost(
            &format!("{}{}", p.system, p.prompt),
            &text,
            p.provider,
            p.model,
        )
    };
    Ok((text, cost))
}

async fn anthropic_call(http: &Client, p: &AgentCallParams<'_>) -> Result<(String, f64)> {
    let key_owned = agent_or_env_api_key(p).ok_or_else(|| anyhow!("No API key for anthropic"))?;
    let key = key_owned.as_str();
    let body = json!({
        "model": p.model,
        "max_tokens": p.max_tokens,
        "system": p.system,
        "messages": [{"role": "user", "content": p.prompt}]
    });
    let resp = http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Anthropic {status}: {txt}"));
    }
    let data: Value = resp.json().await?;
    let text = data["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok((text, 0.0))
}

async fn openai_call(http: &Client, p: &AgentCallParams<'_>, base: &str) -> Result<(String, f64)> {
    let key = agent_or_env_api_key(p);
    let mut last_empty_detail = String::new();
    for attempt in 0..2 {
        let max_tokens = if attempt == 0 {
            p.max_tokens
        } else {
            p.max_tokens.saturating_mul(2).min(16_000)
        };
        let body = openai_request_body(p, max_tokens);
        let mut req = http.post(format!("{base}/chat/completions")).json(&body);
        req = match key.as_deref() {
            Some(key) => req.bearer_auth(key),
            None if crate::ai_local::is_local_openai_base(base) => {
                req.bearer_auth("patchhive-local")
            }
            None => return Err(anyhow!("No API key")),
        };
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI/compat {status}: {txt}"));
        }
        let data: Value = resp.json().await?;
        if let Some(error) = data.get("error") {
            let message = error["message"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| error.to_string());
            return Err(anyhow!("OpenAI/compat provider error: {message}"));
        }
        let text = openai_completion_text(&data);
        if !text.trim().is_empty() {
            return Ok((text, openai_reported_cost(&data)));
        }
        last_empty_detail = openai_empty_detail(&data);
        if attempt == 0 {
            continue;
        }
    }

    Err(anyhow!(
        "OpenAI/compat provider returned an empty completion for model {} ({last_empty_detail})",
        p.model
    ))
}

fn openai_reported_cost(data: &Value) -> f64 {
    data["usage"]["cost"]
        .as_f64()
        .or_else(|| data["usage"]["cost"].as_str()?.parse().ok())
        .unwrap_or(0.0)
}

fn openai_request_body(p: &AgentCallParams<'_>, max_tokens: u32) -> Value {
    let mut body = json!({
            "model": p.model,
            "max_tokens": max_tokens,
            "temperature": 0.1,
            "stream": false,
            "messages": [
                {"role": "system", "content": p.system},
                {"role": "user", "content": p.prompt}
            ]
    });
    if p.provider == "openrouter" {
        body["reasoning"] = json!({"effort": p.reasoning_effort, "exclude": true});
    }
    body
}

fn openai_completion_text(data: &Value) -> String {
    let choice = &data["choices"][0];
    text_content(&choice["message"]["content"])
        .or_else(|| choice["text"].as_str().map(str::to_string))
        .or_else(|| data["output_text"].as_str().map(str::to_string))
        .unwrap_or_default()
}

fn text_content(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let text = parts
                .iter()
                .filter_map(|part| {
                    part.as_str()
                        .map(str::to_string)
                        .or_else(|| part["text"].as_str().map(str::to_string))
                        .or_else(|| part["content"].as_str().map(str::to_string))
                })
                .collect::<Vec<_>>()
                .join("\n");
            (!text.trim().is_empty()).then_some(text)
        }
        Value::Object(_) => value["text"]
            .as_str()
            .or_else(|| value["content"].as_str())
            .map(str::to_string),
        _ => None,
    }
}

fn openai_empty_detail(data: &Value) -> String {
    let choice = &data["choices"][0];
    let finish_reason = choice["finish_reason"].as_str().unwrap_or("unknown");
    let content_shape = match &choice["message"]["content"] {
        Value::Null => "null",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
    };
    let reasoning_tokens = data["usage"]["completion_tokens_details"]["reasoning_tokens"]
        .as_i64()
        .unwrap_or(0);
    format!(
        "finish_reason={finish_reason}, content_shape={content_shape}, reasoning_tokens={reasoning_tokens}"
    )
}

async fn gemini_call(http: &Client, p: &AgentCallParams<'_>) -> Result<(String, f64)> {
    let key_owned = agent_or_env_api_key(p).ok_or_else(|| anyhow!("No Gemini key"))?;
    let key = key_owned.as_str();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        p.model
    );
    let body = json!({
        "system_instruction": {"parts": [{"text": p.system}]},
        "contents": [{"parts": [{"text": p.prompt}]}],
        "generationConfig": {"maxOutputTokens": p.max_tokens}
    });
    let resp = http
        .post(&url)
        .header("x-goog-api-key", key)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini {status}: {txt}"));
    }
    let data: Value = resp.json().await?;
    let text = data["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok((text, 0.0))
}

async fn ollama_call(http: &Client, p: &AgentCallParams<'_>) -> Result<(String, f64)> {
    let base = std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let body = json!({
        "model": p.model,
        "stream": false,
        "options": {"num_predict": p.max_tokens},
        "messages": [
            {"role": "system", "content": p.system},
            {"role": "user", "content": p.prompt}
        ]
    });
    let resp = http
        .post(format!("{base}/api/chat"))
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Ollama {status}: {txt}"));
    }
    let data: Value = resp.json().await?;
    let text = data["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok((text, 0.0))
}

// ── Agent task functions ───────────────────────────────────────────────────────

use crate::state::AgentConfig;

fn call_params<'a>(
    agent: &'a AgentConfig,
    system: &'a str,
    prompt: &'a str,
) -> AgentCallParams<'a> {
    call_params_with_max(agent, system, prompt, DEFAULT_MAX_TOKENS)
}

fn call_params_with_max<'a>(
    agent: &'a AgentConfig,
    system: &'a str,
    prompt: &'a str,
    max_tokens: u32,
) -> AgentCallParams<'a> {
    AgentCallParams {
        provider: agent.provider.as_str(),
        model: agent.model.as_str(),
        base_url: agent.base_url.as_deref(),
        api_key: agent.api_key.as_deref(),
        system,
        prompt,
        max_tokens,
        reasoning_effort: "low",
    }
}

fn scoring_call_params<'a>(
    agent: &'a AgentConfig,
    system: &'a str,
    prompt: &'a str,
) -> AgentCallParams<'a> {
    AgentCallParams {
        reasoning_effort: "none",
        ..call_params(agent, system, prompt)
    }
}

pub async fn agent_score_issues(
    http: &Client,
    issues: &mut [Value],
    agent: &AgentConfig,
) -> Result<f64> {
    let system = "Senior engineer triaging GitHub issues for automated fixing.\n\
        Score each 0-100: +20 clear reproduction, +25 small scope, +20 expected vs actual, \
        +15 definitely a bug, +20 has stacktrace/error/code snippet.\n\
        Reply ONLY with JSON array (no markdown): [{\"id\":<int>,\"score\":<int>,\"reason\":\"<one sentence>\"}]";

    let input: Vec<Value> = issues
        .iter()
        .map(|i| {
            json!({
                "id": i["id"], "number": i["number"], "title": i["title"],
                "body": i["body"].as_str().unwrap_or("").chars().take(400).collect::<String>()
            })
        })
        .collect();

    let prompt = format!("Score:\n{}", serde_json::to_string(&input)?);
    let (text, cost) = ai_call(http, &scoring_call_params(agent, system, &prompt)).await?;
    let scores_arr = parse_json(&text)?;

    let scores: HashMap<i64, Value> = scores_arr
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| s["id"].as_i64().map(|id| (id, s)))
        .collect();

    for issue in issues.iter_mut() {
        if let Some(id) = issue["id"].as_i64() {
            if let Some(s) = scores.get(&id) {
                issue["fixability_score"] = s["score"].clone();
                issue["fixability_reason"] = s["reason"].clone();
            }
        }
    }
    issues.sort_by(|a, b| {
        let sa = a["fixability_score"].as_i64().unwrap_or(0);
        let sb = b["fixability_score"].as_i64().unwrap_or(0);
        sb.cmp(&sa)
    });
    Ok(cost)
}

pub async fn agent_select_files(
    http: &Client,
    structure: &str,
    title: &str,
    body: &str,
    agent: &AgentConfig,
) -> Result<(Vec<String>, f64)> {
    let system = "Software architect. Select ONLY the 3-8 files most relevant to fixing this bug.\nReply ONLY with JSON array of relative paths (no markdown): [\"path/to/file.rs\"]";
    let prompt = format!(
        "Issue: {title}\n\n{}\n\nFiles:\n{structure}",
        &body.chars().take(1000).collect::<String>()
    );
    let (text, cost) = ai_call(http, &call_params(agent, system, &prompt)).await?;
    let files: Vec<String> = parse_typed_json(&text, "judge file selection")?;
    if files.iter().any(|path| path.trim().is_empty()) {
        return Err(anyhow!(
            "Provider response violated the judge file selection contract: paths cannot be empty"
        ));
    }
    Ok((files, cost))
}

pub async fn agent_generate_patch(
    http: &Client,
    title: &str,
    body: &str,
    codebase: &str,
    ctx: &str,
    agent: &AgentConfig,
) -> Result<(GeneratedPatchResponse, f64)> {
    let system = "Expert software engineer. Fix the bug described in the issue.\n\
        Additional context (maintainer comments, linked refs) is provided — use it.\n\
        The patch must be a complete, valid unified diff that `git apply --check` can accept.\n\
        Include full diff headers (`diff --git`, `---`, `+++`, and hunk headers) and do not truncate hunks.\n\
        Reply ONLY with JSON (no markdown):\n\
        {\"explanation\":\"1-2 sentences\",\"files_changed\":[\"path\"],\"patch\":\"<unified diff>\",\"confidence\":0-100}\n\
        confidence = your honest estimate the patch correctly fixes the root cause (0=guessing, 100=certain).\n\
        Set patch to null if you cannot fix it safely.";
    let prompt = format!(
        "Issue: {title}\n\n{}\n\nIssue context:\n{ctx}\n\nCode:\n{codebase}",
        &body.chars().take(1500).collect::<String>()
    );
    let (text, cost) = ai_call(
        http,
        &call_params_with_max(agent, system, &prompt, PATCH_MAX_TOKENS),
    )
    .await?;
    Ok((parse_typed_json(&text, "reaper patch response")?, cost))
}

pub async fn agent_patch_retry(
    http: &Client,
    title: &str,
    body: &str,
    codebase: &str,
    prev_patch: &str,
    error_ctx: &str,
    agent: &AgentConfig,
) -> Result<(PatchRetryResponse, f64)> {
    let system = "Expert software engineer. Previous patch failed — study the error and produce a corrected diff.\n\
        The corrected patch must be a complete, valid unified diff that `git apply --check` can accept.\n\
        Include full diff headers (`diff --git`, `---`, `+++`, and hunk headers) and do not truncate hunks.\n\
        Reply ONLY with JSON (no markdown):\n\
        {\"explanation\":\"what changed vs before\",\"files_changed\":[\"path\"],\"patch\":\"<unified diff>\"}\n\
        Set patch to null if you cannot fix it.";
    let prompt = format!(
        "Issue: {title}\n\n{}\n\nPrevious patch (FAILED):\n{prev_patch}\n\nFailure:\n{error_ctx}\n\nCode:\n{codebase}",
        &body.chars().take(1000).collect::<String>()
    );
    let (text, cost) = ai_call(
        http,
        &call_params_with_max(agent, system, &prompt, PATCH_MAX_TOKENS),
    )
    .await?;
    Ok((parse_typed_json(&text, "patch retry response")?, cost))
}

pub async fn agent_smith_patch(
    http: &Client,
    title: &str,
    patch: &str,
    explanation: &str,
    agent: &AgentConfig,
) -> Result<(SmithReviewResponse, f64)> {
    let system = "Senior code reviewer. Does this patch correctly and safely fix the bug?\n\
        Reply ONLY with JSON (no markdown):\n\
        {\"approved\":true/false,\"confidence\":0-100,\"feedback\":\"brief\",\"improved_patch\":\"<diff or null>\"}";
    let prompt = format!("Issue: {title}\nFix: {explanation}\nPatch:\n{patch}");
    let (text, cost) = ai_call(
        http,
        &call_params_with_max(agent, system, &prompt, REVIEW_MAX_TOKENS),
    )
    .await?;
    Ok((parse_typed_json(&text, "smith review response")?, cost))
}

pub async fn agent_dry_run_analysis(
    http: &Client,
    issues: &[Value],
    repos: &[Value],
    agent: &AgentConfig,
) -> (Result<DryRunAnalysisResponse>, f64) {
    let system = "Senior engineer reviewing GitHub issues for automated patching.\n\
        Reply ONLY with JSON (no markdown, no prose). Use this exact shape:\n\
        {\"summary\":\"max 18 words\",\"top_candidate\":{\"repo\":\"owner/name\",\"title\":\"issue title\",\"score\":0,\"why\":\"max 18 words\"},\
        \"success_band\":\"low|medium|high\",\"risk\":\"max 18 words\",\
        \"recommendation\":\"skip|manual review|dry-run only|safe to attempt\",\
        \"candidates\":[{\"repo\":\"owner/name\",\"title\":\"issue title\",\"score\":0,\"call\":\"skip|review|attempt\",\"reason\":\"max 14 words\"}]}\n\
        Keep candidates to the best 3 and stay conservative.";
    let repo_names: Vec<&str> = repos
        .iter()
        .filter_map(|r| r["full_name"].as_str())
        .take(5)
        .collect();
    let issue_list: Vec<String> = issues
        .iter()
        .take(20)
        .map(|i| {
            format!(
                "- #{} [{}/100] {}: {}",
                i["number"].as_i64().unwrap_or(0),
                i["fixability_score"].as_i64().unwrap_or(50),
                i["repo"].as_str().unwrap_or(""),
                i["title"].as_str().unwrap_or("")
            )
        })
        .collect();
    let prompt = format!(
        "Repos ({}): {}\n\nIssues ({}):\n{}",
        repo_names.len(),
        repo_names.join(", "),
        issues.len(),
        issue_list.join("\n")
    );
    let (text, mut cost) = match ai_call(http, &call_params(agent, system, &prompt)).await {
        Ok(response) => response,
        Err(error) => return (Err(error), 0.0),
    };
    match parse_typed_json(&text, "dry-run scout analysis") {
        Ok(report) => (Ok(report), cost),
        Err(first_error) => {
            let repair_system = "Repair one malformed dry-run analysis response. Reply ONLY with one complete JSON object matching the requested schema. Preserve the original facts, keep candidates to at most 3, and do not add markdown.";
            let repair_prompt = format!(
                "The first response failed validation: {first_error}\n\nRequired schema:\n{system}\n\nMalformed response:\n{}",
                text.chars().take(8_000).collect::<String>()
            );
            let (repaired, repair_cost) = match ai_call(
                http,
                &call_params_with_max(agent, repair_system, &repair_prompt, 3_000),
            )
            .await
            {
                Ok(response) => response,
                Err(error) => return (Err(error), cost),
            };
            cost += repair_cost;
            (
                parse_typed_json(&repaired, "repaired dry-run scout analysis"),
                cost,
            )
        }
    }
}

pub async fn agent_pr_comment_fix(
    http: &Client,
    issue_title: &str,
    maintainer_comment: &str,
    codebase: &str,
    agent: &AgentConfig,
) -> Result<(Value, f64)> {
    let system = "Expert software engineer. A maintainer says the previous fix is wrong.\n\
        Read their feedback carefully and produce a corrected patch.\n\
        Reply ONLY with JSON (no markdown):\n\
        {\"explanation\":\"what changed and why\",\"files_changed\":[\"path\"],\"patch\":\"<unified diff>\"}";
    let prompt = format!(
        "Original issue: {issue_title}\nMaintainer: {maintainer_comment}\nCode:\n{codebase}"
    );
    let (text, cost) = ai_call(
        http,
        &call_params_with_max(agent, system, &prompt, PATCH_MAX_TOKENS),
    )
    .await?;
    Ok((parse_json(&text)?, cost))
}

#[cfg(test)]
mod tests {
    use super::{
        openai_reported_cost, openai_request_body, parse_typed_json, AgentCallParams,
        DryRunAnalysisResponse, GeneratedPatchResponse, SmithReviewResponse,
    };

    #[test]
    fn generated_patch_contract_requires_confidence_and_patch_field() {
        let missing_confidence =
            r#"{"explanation":"fix","files_changed":["src/lib.rs"],"patch":"diff"}"#;
        let missing_patch =
            r#"{"explanation":"fix","files_changed":["src/lib.rs"],"confidence":80}"#;

        assert!(parse_typed_json::<GeneratedPatchResponse>(
            missing_confidence,
            "reaper patch response"
        )
        .unwrap_err()
        .to_string()
        .contains("missing field `confidence`"));
        assert!(
            parse_typed_json::<GeneratedPatchResponse>(missing_patch, "reaper patch response")
                .unwrap_err()
                .to_string()
                .contains("missing field `patch`")
        );
    }

    #[test]
    fn nullable_patch_and_improved_patch_are_valid_typed_outcomes() {
        let patch: GeneratedPatchResponse = parse_typed_json(
            r#"{"explanation":"unsafe","files_changed":[],"patch":null,"confidence":12}"#,
            "reaper patch response",
        )
        .expect("nullable generated patch");
        let review: SmithReviewResponse = parse_typed_json(
            r#"{"approved":false,"confidence":25,"feedback":"hold","improved_patch":null}"#,
            "smith review response",
        )
        .expect("nullable improved patch");

        assert!(patch.patch.is_none());
        assert!(review.improved_patch.is_none());
    }

    #[test]
    fn dry_run_contract_rejects_incomplete_nested_candidates() {
        let incomplete = r#"{
            "summary":"summary",
            "top_candidate":{"repo":"acme/repo","title":"bug","score":80},
            "success_band":"high",
            "risk":"low",
            "recommendation":"safe to attempt",
            "candidates":[]
        }"#;

        assert!(
            parse_typed_json::<DryRunAnalysisResponse>(incomplete, "dry-run scout analysis")
                .unwrap_err()
                .to_string()
                .contains("missing field `why`")
        );
    }

    #[test]
    fn openrouter_request_reserves_output_space_with_low_reasoning() {
        let mut params = AgentCallParams {
            provider: "openrouter",
            model: "cohere/north-mini-code:free",
            base_url: None,
            api_key: None,
            system: "Return JSON",
            prompt: "Score issues",
            max_tokens: 2_000,
            reasoning_effort: "low",
        };

        let body = openai_request_body(&params, 4_000);

        assert_eq!(body["max_tokens"], 4_000);
        assert_eq!(body["reasoning"]["effort"], "low");
        assert_eq!(body["reasoning"]["exclude"], true);

        params.reasoning_effort = "none";
        let scoring_body = openai_request_body(&params, 2_000);
        assert_eq!(scoring_body["reasoning"]["effort"], "none");
    }

    #[test]
    fn openrouter_cost_uses_provider_usage_value() {
        assert_eq!(
            openai_reported_cost(&serde_json::json!({"usage": {"cost": 0.0123}})),
            0.0123
        );
        assert_eq!(
            openai_reported_cost(&serde_json::json!({"usage": {"cost": "0"}})),
            0.0
        );
    }
}
