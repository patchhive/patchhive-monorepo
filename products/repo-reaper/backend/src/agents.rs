// agents.rs — Multi-provider AI calls for all RepoReaper agent roles
// Uses direct HTTP (reqwest) for full provider control.
// yoagent is used in Praxis for the full agent loop; here we do one-shot completions.

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;
use once_cell::sync::Lazy;

// ── Cost table ($/1k tokens: input, output) ───────────────────────────────────
fn cost_rates(provider: &str, model: &str) -> (f64, f64) {
    match (provider, model) {
        ("anthropic", m) if m.contains("opus")   => (0.015, 0.075),
        ("anthropic", m) if m.contains("sonnet") => (0.003, 0.015),
        ("anthropic", _)                          => (0.00025, 0.00125),
        ("openai", m) if m.contains("gpt-4o") && !m.contains("mini") => (0.0025, 0.01),
        ("openai", m) if m.contains("mini")      => (0.00015, 0.0006),
        ("openai", _)                             => (0.0025, 0.01),
        ("gemini", _)                             => (0.00035, 0.00105),
        ("groq", _)                               => (0.00059, 0.00079),
        ("ollama", _)                             => (0.0, 0.0),
        _                                         => (0.003, 0.015),
    }
}

fn estimate_cost(prompt: &str, response: &str, provider: &str, model: &str) -> f64 {
    let (ic, oc) = cost_rates(provider, model);
    (prompt.len() as f64 / 4.0 / 1000.0) * ic + (response.len() as f64 / 4.0 / 1000.0) * oc
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
    serde_json::from_str(clean).map_err(|e| anyhow!("JSON parse error: {e}\nRaw: {text}"))
}

// ── Provider cooldowns ─────────────────────────────────────────────────────────

static COOLDOWNS: Lazy<RwLock<HashMap<String, Instant>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub async fn provider_available(provider: &str) -> bool {
    let map = COOLDOWNS.read().await;
    map.get(provider).map(|t| Instant::now() >= *t).unwrap_or(true)
}

pub async fn set_cooldown(provider: &str, secs: u64) {
    let mut map = COOLDOWNS.write().await;
    map.insert(provider.to_string(), Instant::now() + Duration::from_secs(secs));
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
    pub api_key: Option<&'a str>,
    pub bot_user: Option<&'a str>,
    pub system: &'a str,
    pub prompt: &'a str,
}

pub async fn ai_call(http: &Client, p: &AgentCallParams<'_>) -> Result<(String, f64)> {
    if !provider_available(p.provider).await {
        return Err(anyhow!("Provider {} is cooling down", p.provider));
    }

    let result = match p.provider {
        "anthropic" => anthropic_call(http, p).await,
        "openai"    => {
            let base = crate::ai_local::openai_base_url();
            openai_call(http, p, &base).await
        }
        "gemini"    => gemini_call(http, p).await,
        "groq"      => {
            let base = std::env::var("GROQ_BASE_URL")
                .unwrap_or_else(|_| "https://api.groq.com/openai/v1".into());
            openai_call(http, p, &base).await
        }
        "ollama"    => ollama_call(http, p).await,
        _           => Err(anyhow!("Unknown provider: {}", p.provider)),
    };

    if let Err(ref e) = result {
        let msg = e.to_string().to_lowercase();
        if msg.contains("rate limit") || msg.contains("429") || msg.contains("quota") {
            set_cooldown(p.provider, 90).await;
        }
    }

    let (text, _cost) = result?;
    let cost = estimate_cost(
        &format!("{}{}", p.system, p.prompt),
        &text,
        p.provider,
        p.model,
    );
    Ok((text, cost))
}

async fn anthropic_call(http: &Client, p: &AgentCallParams<'_>) -> Result<(String, f64)> {
    let key_owned = p.api_key.map(|s| s.to_string())
        .or_else(|| std::env::var("PROVIDER_API_KEY").ok())
        .ok_or_else(|| anyhow!("No API key for anthropic"))?;
    let key = key_owned.as_str();
    let body = json!({
        "model": p.model,
        "max_tokens": 2000,
        "system": p.system,
        "messages": [{"role": "user", "content": p.prompt}]
    });
    let resp = http.post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Anthropic {status}: {txt}"));
    }
    let data: Value = resp.json().await?;
    let text = data["content"][0]["text"].as_str().unwrap_or("").to_string();
    Ok((text, 0.0))
}

async fn openai_call(http: &Client, p: &AgentCallParams<'_>, base: &str) -> Result<(String, f64)> {
    let key = p.api_key
        .map(|s| s.to_string())
        .or_else(|| std::env::var("PROVIDER_API_KEY").ok())
        .filter(|value| !value.trim().is_empty());
    let body = json!({
        "model": p.model,
        "max_tokens": 2000,
        "messages": [
            {"role": "system", "content": p.system},
            {"role": "user", "content": p.prompt}
        ]
    });
    let mut req = http.post(format!("{base}/chat/completions")).json(&body);
    req = match key.as_deref() {
        Some(key) => req.bearer_auth(key),
        None if crate::ai_local::is_local_openai_base(base) => req.bearer_auth("patchhive-local"),
        None => return Err(anyhow!("No API key")),
    };
    let resp = req.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(anyhow!("OpenAI/compat {status}: {txt}"));
    }
    let data: Value = resp.json().await?;
    let text = data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
    Ok((text, 0.0))
}

async fn gemini_call(http: &Client, p: &AgentCallParams<'_>) -> Result<(String, f64)> {
    let key_owned = p.api_key.map(|s| s.to_string()).or_else(|| std::env::var("PROVIDER_API_KEY").ok()).ok_or_else(|| anyhow!("No Gemini key"))?;
    let key = key_owned.as_str();
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", p.model, key);
    let body = json!({
        "system_instruction": {"parts": [{"text": p.system}]},
        "contents": [{"parts": [{"text": p.prompt}]}],
        "generationConfig": {"maxOutputTokens": 2000}
    });
    let resp = http.post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini {status}: {txt}"));
    }
    let data: Value = resp.json().await?;
    let text = data["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("").to_string();
    Ok((text, 0.0))
}

async fn ollama_call(http: &Client, p: &AgentCallParams<'_>) -> Result<(String, f64)> {
    let base = std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let body = json!({
        "model": p.model,
        "stream": false,
        "messages": [
            {"role": "system", "content": p.system},
            {"role": "user", "content": p.prompt}
        ]
    });
    let resp = http.post(format!("{base}/api/chat")).json(&body).send().await?;
    let data: Value = resp.json().await?;
    let text = data["message"]["content"].as_str().unwrap_or("").to_string();
    Ok((text, 0.0))
}

// ── Agent task functions ───────────────────────────────────────────────────────

use crate::state::AgentConfig;

fn call_params<'a>(agent: &'a AgentConfig, system: &'a str, prompt: &'a str) -> AgentCallParams<'a> {
    AgentCallParams {
        provider: agent.provider.as_str(),
        model: agent.model.as_str(),
        api_key: agent.api_key.as_deref(),
        bot_user: agent.bot_user.as_deref(),
        system,
        prompt,
    }
}

pub async fn agent_score_issues(http: &Client, issues: &mut Vec<Value>, agent: &AgentConfig) -> Result<f64> {
    let system = "Senior engineer triaging GitHub issues for automated fixing.\n\
        Score each 0-100: +20 clear reproduction, +25 small scope, +20 expected vs actual, \
        +15 definitely a bug, +20 has stacktrace/error/code snippet.\n\
        Reply ONLY with JSON array (no markdown): [{\"id\":<int>,\"score\":<int>,\"reason\":\"<one sentence>\"}]";

    let input: Vec<Value> = issues.iter().map(|i| json!({
        "id": i["id"], "number": i["number"], "title": i["title"],
        "body": i["body"].as_str().unwrap_or("").chars().take(400).collect::<String>()
    })).collect();

    let prompt = format!("Score:\n{}", serde_json::to_string(&input)?);
    let (text, cost) = ai_call(http, &call_params(agent, system, &prompt)).await?;
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

pub async fn agent_select_files(http: &Client, structure: &str, title: &str, body: &str, agent: &AgentConfig) -> Result<(Vec<String>, f64)> {
    let system = "Software architect. Select ONLY the 3-8 files most relevant to fixing this bug.\nReply ONLY with JSON array of relative paths (no markdown): [\"path/to/file.rs\"]";
    let prompt = format!("Issue: {title}\n\n{}\n\nFiles:\n{structure}", &body.chars().take(1000).collect::<String>());
    let (text, cost) = ai_call(http, &call_params(agent, system, &prompt)).await?;
    let parsed = parse_json(&text)?;
    let files = parsed
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    Ok((files, cost))
}

pub async fn agent_generate_patch(http: &Client, title: &str, body: &str, codebase: &str, ctx: &str, agent: &AgentConfig) -> Result<(Value, f64)> {
    let system = "Expert software engineer. Fix the bug described in the issue.\n\
        Additional context (maintainer comments, linked refs) is provided — use it.\n\
        Reply ONLY with JSON (no markdown):\n\
        {\"explanation\":\"1-2 sentences\",\"files_changed\":[\"path\"],\"patch\":\"<unified diff>\",\"confidence\":0-100}\n\
        confidence = your honest estimate the patch correctly fixes the root cause (0=guessing, 100=certain).\n\
        Set patch to null if you cannot fix it safely.";
    let prompt = format!(
        "Issue: {title}\n\n{}\n\nIssue context:\n{ctx}\n\nCode:\n{codebase}",
        &body.chars().take(1500).collect::<String>()
    );
    let (text, cost) = ai_call(http, &call_params(agent, system, &prompt)).await?;
    Ok((parse_json(&text)?, cost))
}

pub async fn agent_patch_retry(http: &Client, title: &str, body: &str, codebase: &str, prev_patch: &str, error_ctx: &str, agent: &AgentConfig) -> Result<(Value, f64)> {
    let system = "Expert software engineer. Previous patch failed — study the error and produce a corrected diff.\n\
        Reply ONLY with JSON (no markdown):\n\
        {\"explanation\":\"what changed vs before\",\"files_changed\":[\"path\"],\"patch\":\"<unified diff>\"}\n\
        Set patch to null if you cannot fix it.";
    let prompt = format!(
        "Issue: {title}\n\n{}\n\nPrevious patch (FAILED):\n{prev_patch}\n\nFailure:\n{error_ctx}\n\nCode:\n{codebase}",
        &body.chars().take(1000).collect::<String>()
    );
    let (text, cost) = ai_call(http, &call_params(agent, system, &prompt)).await?;
    Ok((parse_json(&text)?, cost))
}

pub async fn agent_smith_patch(http: &Client, title: &str, patch: &str, explanation: &str, agent: &AgentConfig) -> Result<(Value, f64)> {
    let system = "Senior code reviewer. Does this patch correctly and safely fix the bug?\n\
        Reply ONLY with JSON (no markdown):\n\
        {\"approved\":true/false,\"confidence\":0-100,\"feedback\":\"brief\",\"improved_patch\":\"<diff or null>\"}";
    let prompt = format!("Issue: {title}\nFix: {explanation}\nPatch:\n{patch}");
    let (text, cost) = ai_call(http, &call_params(agent, system, &prompt)).await?;
    Ok((parse_json(&text)?, cost))
}

pub async fn agent_dry_run_analysis(http: &Client, issues: &[Value], repos: &[Value], agent: &AgentConfig) -> Result<(String, f64)> {
    let system = "Senior engineer reviewing GitHub issues for automated patching.\n\
        Produce a concise report: which issues look most reapable, likely complexity, \
        expected success rate, potential risks. Be specific and practical.";
    let repo_names: Vec<&str> = repos.iter().filter_map(|r| r["full_name"].as_str()).take(5).collect();
    let issue_list: Vec<String> = issues.iter().take(20).map(|i| {
        format!("- #{} [{}/100] {}: {}",
            i["number"].as_i64().unwrap_or(0),
            i["fixability_score"].as_i64().unwrap_or(50),
            i["repo"].as_str().unwrap_or(""),
            i["title"].as_str().unwrap_or(""))
    }).collect();
    let prompt = format!("Repos ({}): {}\n\nIssues ({}):\n{}", repo_names.len(), repo_names.join(", "), issues.len(), issue_list.join("\n"));
    let (text, cost) = ai_call(http, &call_params(agent, system, &prompt)).await?;
    Ok((text, cost))
}

pub async fn agent_pr_comment_fix(http: &Client, issue_title: &str, maintainer_comment: &str, codebase: &str, agent: &AgentConfig) -> Result<(Value, f64)> {
    let system = "Expert software engineer. A maintainer says the previous fix is wrong.\n\
        Read their feedback carefully and produce a corrected patch.\n\
        Reply ONLY with JSON (no markdown):\n\
        {\"explanation\":\"what changed and why\",\"files_changed\":[\"path\"],\"patch\":\"<unified diff>\"}";
    let prompt = format!("Original issue: {issue_title}\nMaintainer: {maintainer_comment}\nCode:\n{codebase}");
    let (text, cost) = ai_call(http, &call_params(agent, system, &prompt)).await?;
    Ok((parse_json(&text)?, cost))
}
