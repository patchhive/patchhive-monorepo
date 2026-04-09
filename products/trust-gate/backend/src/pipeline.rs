use std::collections::BTreeSet;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use glob::Pattern;
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use uuid::Uuid;

use crate::{
    db, github,
    models::{
        FileAssessment, GitHubPrReviewRequest, GitHubReviewContext, RepoRuleSet, ReviewFinding,
        ReviewHistoryItem, ReviewMetricSummary, ReviewRequest, ReviewResult, RulePack,
    },
    state::AppState,
};

type ApiError = (StatusCode, Json<serde_json::Value>);

#[derive(Debug, Default)]
struct FilePatch {
    path: String,
    additions: u32,
    deletions: u32,
    added_lines: Vec<String>,
}

fn api_error(status: StatusCode, message: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": message.into() })))
}

fn matches_rule(value: &str, pattern: &str) -> bool {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.contains('*') || trimmed.contains('?') || trimmed.contains('[') {
        return Pattern::new(trimmed)
            .map(|compiled| compiled.matches(value))
            .unwrap_or_else(|_| value.to_lowercase().contains(&trimmed.to_lowercase()));
    }

    value.to_lowercase().contains(&trimmed.to_lowercase())
}

fn matching_patterns<'a>(value: &str, patterns: &'a [String]) -> Vec<&'a str> {
    patterns
        .iter()
        .map(String::as_str)
        .filter(|pattern| matches_rule(value, pattern))
        .collect()
}

fn parse_diff(diff: &str) -> Vec<FilePatch> {
    let mut files = Vec::new();
    let mut current: Option<FilePatch> = None;
    let mut fragment_counter = 1u32;

    let flush = |files: &mut Vec<FilePatch>, current: &mut Option<FilePatch>| {
        if let Some(mut patch) = current.take() {
            if patch.path.trim().is_empty() {
                patch.path = format!("diff-fragment-{}", files.len() + 1);
            }
            files.push(patch);
        }
    };

    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            flush(&mut files, &mut current);
            let path = line
                .split_whitespace()
                .nth(3)
                .map(|value| value.trim_start_matches("b/").to_string())
                .unwrap_or_else(|| format!("diff-fragment-{fragment_counter}"));
            fragment_counter += 1;
            current = Some(FilePatch {
                path,
                ..FilePatch::default()
            });
            continue;
        }

        if line.starts_with("+++ b/") {
            let path = line.trim_start_matches("+++ b/").trim().to_string();
            if let Some(current_patch) = current.as_mut() {
                current_patch.path = path;
            } else {
                current = Some(FilePatch {
                    path,
                    ..FilePatch::default()
                });
            }
            continue;
        }

        if !line.trim().is_empty() && current.is_none() {
            current = Some(FilePatch {
                path: format!("diff-fragment-{fragment_counter}"),
                ..FilePatch::default()
            });
            fragment_counter += 1;
        }

        if let Some(current_patch) = current.as_mut() {
            if line.starts_with('+') && !line.starts_with("+++") {
                current_patch.additions += 1;
                current_patch
                    .added_lines
                    .push(line.trim_start_matches('+').to_string());
            } else if line.starts_with('-') && !line.starts_with("---") {
                current_patch.deletions += 1;
            }
        }
    }

    flush(&mut files, &mut current);
    files
}

fn clamp_score(value: usize) -> u32 {
    value.min(100) as u32
}

fn make_finding(
    key: &str,
    label: &str,
    severity: &str,
    detail: impl Into<String>,
    evidence: Vec<String>,
) -> ReviewFinding {
    ReviewFinding {
        key: key.into(),
        label: label.into(),
        severity: severity.into(),
        detail: detail.into(),
        evidence,
    }
}

fn is_generated_path(path: &str) -> bool {
    [
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "Cargo.lock",
        "*.min.js",
        "*.map",
        "*.snap",
        "*.pb.go",
        "*.generated.*",
        "*.g.dart",
        "dist/",
        "build/",
        "coverage/",
        "vendor/",
        "generated/",
    ]
    .iter()
    .any(|pattern| matches_rule(path, pattern))
}

fn is_docs_path(path: &str) -> bool {
    [
        "README",
        ".md",
        "docs/",
        "CHANGELOG",
        "LICENSE",
        ".txt",
    ]
    .iter()
    .any(|pattern| matches_rule(path, pattern))
}

fn path_policy_note(path: &str) -> Option<&'static str> {
    if matches_rule(path, ".github/workflows/") {
        Some("Workflow changes can alter CI execution, release behavior, or secret exposure.")
    } else if matches_rule(path, "infra/") || matches_rule(path, "terraform/") {
        Some("Infrastructure changes can affect deployment safety, runtime access, or production posture.")
    } else if matches_rule(path, "migrations/") || matches_rule(path, "schema.sql") {
        Some("Database or schema changes deserve extra review because rollback and data safety can be expensive.")
    } else if matches_rule(path, "auth/") || matches_rule(path, "permissions") {
        Some("Authentication and permission changes need deliberate human review.")
    } else if matches_rule(path, "billing") {
        Some("Billing-related changes can have customer or revenue impact if they drift.")
    } else if matches_rule(path, "Dockerfile") || matches_rule(path, "docker-compose") {
        Some("Container/runtime changes can shift the execution environment in subtle ways.")
    } else {
        None
    }
}

fn rule_packs_catalog() -> Vec<RulePack> {
    let mut app = RepoRuleSet::default();
    app.max_files = 14;
    app.max_additions = 520;
    app.max_deletions = 320;
    app.warn_paths.extend(["config/".into(), "routes/".into(), "api/".into()]);
    app.notes = "Balanced defaults for product apps: guard auth, data, runtime, and deployment-sensitive paths.".into();

    let mut library = RepoRuleSet::default();
    library.max_files = 10;
    library.max_additions = 280;
    library.max_deletions = 200;
    library.warn_paths.extend(["Cargo.toml".into(), "package.json".into(), "exports".into()]);
    library.require_test_for_paths.extend(["crates/".into(), "packages/".into()]);
    library.notes = "Stricter defaults for reusable libraries where public API drift and missing tests hurt downstream users.".into();

    let mut infra = RepoRuleSet::default();
    infra.blocked_paths.extend([
        "k8s/".into(),
        "helm/".into(),
        "*.tf".into(),
        "*.tfvars".into(),
        "ansible/".into(),
    ]);
    infra.warn_paths.extend(["deploy/".into(), "ops/".into()]);
    infra.suspicious_terms.extend(["0.0.0.0/0".into(), "allow all".into()]);
    infra.max_files = 8;
    infra.max_additions = 260;
    infra.max_deletions = 180;
    infra.notes = "Infra repos get tighter budgets because config drift can fan out quickly.".into();

    let mut agent_patch = RepoRuleSet::default();
    agent_patch.warn_paths.extend(["scripts/".into(), "release".into(), "ci".into()]);
    agent_patch.max_files = 8;
    agent_patch.max_additions = 220;
    agent_patch.max_deletions = 140;
    agent_patch.notes =
        "Strict defaults for agent-generated patches. Keep scope small and avoid workflow or infra churn."
            .into();

    vec![
        RulePack {
            id: "app".into(),
            label: "Product App".into(),
            description: "Balanced guardrails for web or service products with user-facing code, auth, and runtime concerns.".into(),
            rules: app,
        },
        RulePack {
            id: "library".into(),
            label: "Library".into(),
            description: "Tighter scope caps and stronger test expectations for reusable packages and crates.".into(),
            rules: library,
        },
        RulePack {
            id: "infra".into(),
            label: "Infra".into(),
            description: "Treat deployment and infrastructure churn as high risk with smaller allowed patches.".into(),
            rules: infra,
        },
        RulePack {
            id: "agent-patch".into(),
            label: "Agent Patch Repo".into(),
            description: "Conservative defaults for autonomous patching flows that must earn trust incrementally.".into(),
            rules: agent_patch,
        },
    ]
}

fn resolve_rules(repo: &str, inline_rules: Option<RepoRuleSet>) -> Result<RepoRuleSet, ApiError> {
    let mut rules = if let Some(mut rules) = inline_rules {
        rules.repo = repo.to_string();
        rules
    } else if let Some(saved) = db::get_rules(repo)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
    {
        saved
    } else {
        let mut defaults = RepoRuleSet::default();
        defaults.repo = repo.to_string();
        defaults
    };

    rules.repo = repo.to_string();
    Ok(rules)
}

fn review_diff(
    repo: &str,
    diff: &str,
    ai_source: &str,
    rules: RepoRuleSet,
    source_kind: &str,
    github_context: Option<GitHubReviewContext>,
) -> ReviewResult {
    let patches = parse_diff(diff);
    let files_changed = patches.len() as u32;
    let additions = patches.iter().map(|patch| patch.additions).sum::<u32>();
    let deletions = patches.iter().map(|patch| patch.deletions).sum::<u32>();

    let mut findings = Vec::new();
    let mut files = Vec::new();
    let mut blocked_path_hits = Vec::new();
    let mut warn_path_hits = Vec::new();
    let mut blocked_term_hits = Vec::new();
    let mut suspicious_term_hits = Vec::new();
    let mut generated_hits = Vec::new();
    let mut policy_hits = Vec::new();
    let mut files_requiring_tests = Vec::new();
    let mut tests_changed = 0u32;
    let mut source_files_changed = 0u32;
    let mut generated_files = 0u32;
    let mut sensitive_source_files = 0u32;

    for patch in patches {
        let mut status = "safe".to_string();
        let mut reasons = Vec::new();
        let mut matched_rules = Vec::new();

        let is_generated = is_generated_path(&patch.path);
        let is_docs = is_docs_path(&patch.path);
        let is_test = !matching_patterns(&patch.path, &rules.test_paths).is_empty();
        let requires_tests =
            !is_generated
                && !is_docs
                && !is_test
                && !matching_patterns(&patch.path, &rules.require_test_for_paths).is_empty();

        if is_test {
            tests_changed += 1;
        }

        if is_generated {
            generated_files += 1;
            generated_hits.push(patch.path.clone());
            matched_rules.push("generated artifact".into());
            reasons.push(
                "Likely generated or lockfile output. Review the source-of-truth change, not only the generated file."
                    .into(),
            );
        }

        if requires_tests {
            source_files_changed += 1;
            files_requiring_tests.push(patch.path.clone());
            matched_rules.push("test coverage expected".into());
        }

        let blocked_paths = matching_patterns(&patch.path, &rules.blocked_paths);
        if !blocked_paths.is_empty() {
            status = "block".into();
            reasons.push("Touches a blocked path".to_string());
            matched_rules.extend(
                blocked_paths
                    .iter()
                    .map(|pattern| format!("blocked path: {pattern}")),
            );
            blocked_path_hits.push(format!("{} ({})", patch.path, blocked_paths.join(", ")));
        }

        let warn_paths = matching_patterns(&patch.path, &rules.warn_paths);
        if !warn_paths.is_empty() {
            if status != "block" {
                status = "warn".into();
            }
            reasons.push("Touches a sensitive path".to_string());
            matched_rules.extend(
                warn_paths
                    .iter()
                    .map(|pattern| format!("warn path: {pattern}")),
            );
            warn_path_hits.push(format!("{} ({})", patch.path, warn_paths.join(", ")));
            if requires_tests {
                sensitive_source_files += 1;
            }
        }

        let path_policy = path_policy_note(&patch.path).unwrap_or("").to_string();
        if !path_policy.is_empty() {
            if status == "safe" {
                status = "warn".into();
            }
            reasons.push(path_policy.clone());
            policy_hits.push(format!("{} -> {}", patch.path, path_policy));
        }

        for term in &rules.blocked_terms {
            let matches = patch
                .added_lines
                .iter()
                .filter(|line| line.to_lowercase().contains(&term.to_lowercase()))
                .take(2)
                .cloned()
                .collect::<Vec<_>>();
            if !matches.is_empty() {
                status = "block".into();
                reasons.push(format!("Added blocked term '{term}'"));
                matched_rules.push(format!("blocked term: {term}"));
                blocked_term_hits.push(format!("{} -> {}", patch.path, matches.join(" | ")));
            }
        }

        for term in &rules.suspicious_terms {
            let matches = patch
                .added_lines
                .iter()
                .filter(|line| line.to_lowercase().contains(&term.to_lowercase()))
                .take(2)
                .cloned()
                .collect::<Vec<_>>();
            if !matches.is_empty() {
                if status == "safe" {
                    status = "warn".into();
                }
                reasons.push(format!("Added suspicious term '{term}'"));
                matched_rules.push(format!("suspicious term: {term}"));
                suspicious_term_hits.push(format!("{} -> {}", patch.path, matches.join(" | ")));
            }
        }

        if is_generated && patch.additions + patch.deletions > 120 && status == "safe" {
            status = "warn".into();
        }

        let summary = if reasons.is_empty() {
            "No immediate rule hits in this file.".to_string()
        } else {
            reasons.join(" ")
        };

        files.push(FileAssessment {
            path: patch.path,
            status,
            additions: patch.additions,
            deletions: patch.deletions,
            matched_rules,
            summary,
            generated: is_generated,
            path_policy,
        });
    }

    files.sort_by_key(|file| match file.status.as_str() {
        "block" => 0,
        "warn" => 1,
        _ => 2,
    });

    if !blocked_path_hits.is_empty() {
        findings.push(make_finding(
            "blocked_paths",
            "Blocked file paths",
            "block",
            "The diff touches file areas that should not move forward without explicit review.",
            blocked_path_hits.into_iter().take(6).collect(),
        ));
    }

    if !warn_path_hits.is_empty() {
        findings.push(make_finding(
            "warn_paths",
            "Sensitive file paths",
            "warn",
            "The diff touches file areas that deserve extra scrutiny.",
            warn_path_hits.into_iter().take(6).collect(),
        ));
    }

    if !policy_hits.is_empty() {
        findings.push(make_finding(
            "path_policy",
            "Path-specific policy warnings",
            "warn",
            "TrustGate matched one or more repo-sensitive policy zones such as CI, auth, data, or infrastructure paths.",
            policy_hits.into_iter().take(6).collect(),
        ));
    }

    if !blocked_term_hits.is_empty() {
        findings.push(make_finding(
            "blocked_terms",
            "Blocked added content",
            "block",
            "The diff appears to add secret-like or explicitly banned content.",
            blocked_term_hits.into_iter().take(6).collect(),
        ));
    }

    if !suspicious_term_hits.is_empty() {
        findings.push(make_finding(
            "suspicious_terms",
            "Suspicious added content",
            "warn",
            "The diff adds lines that often correlate with fragile or risky changes.",
            suspicious_term_hits.into_iter().take(6).collect(),
        ));
    }

    if !generated_hits.is_empty() && source_files_changed == 0 {
        findings.push(make_finding(
            "generated_only",
            "Generated artifacts without source context",
            "warn",
            "This diff mostly touches generated or lockfile artifacts. Review the source-of-truth change that produced them before merging.",
            generated_hits.into_iter().take(6).collect(),
        ));
    }

    let files_budget_exceeded = files_changed > rules.max_files;
    let additions_budget_exceeded = additions > rules.max_additions;
    let deletions_budget_exceeded = deletions > rules.max_deletions;

    if files_budget_exceeded {
        let severity = if files_changed > rules.max_files.saturating_mul(2) {
            "block"
        } else {
            "warn"
        };
        findings.push(make_finding(
            "scope_files",
            "Scope exceeds file budget",
            severity,
            format!(
                "This diff changes {files_changed} files, above the repo rule limit of {}.",
                rules.max_files
            ),
            vec![format!("{files_changed} changed files")],
        ));
    }

    if additions_budget_exceeded {
        let severity = if additions > rules.max_additions.saturating_mul(2) {
            "block"
        } else {
            "warn"
        };
        findings.push(make_finding(
            "scope_additions",
            "Additions exceed budget",
            severity,
            format!(
                "This diff adds {additions} lines, above the repo rule limit of {}.",
                rules.max_additions
            ),
            vec![format!("{additions} added lines")],
        ));
    }

    if deletions_budget_exceeded {
        let severity = if deletions > rules.max_deletions.saturating_mul(2) {
            "block"
        } else {
            "warn"
        };
        findings.push(make_finding(
            "scope_deletions",
            "Deletions exceed budget",
            severity,
            format!(
                "This diff deletes {deletions} lines, above the repo rule limit of {}.",
                rules.max_deletions
            ),
            vec![format!("{deletions} deleted lines")],
        ));
    }

    if !files_requiring_tests.is_empty() && tests_changed == 0 {
        let severity = if files_requiring_tests.len() >= 3
            || additions > 120
            || deletions > 80
            || sensitive_source_files > 0
        {
            "block"
        } else {
            "warn"
        };

        findings.push(make_finding(
            "missing_tests",
            "Code changes without tests",
            severity,
            "The diff touches code paths that normally deserve tests, but no test files changed.",
            files_requiring_tests
                .iter()
                .take(6)
                .cloned()
                .collect::<Vec<_>>(),
        ));
    }

    let risky_files = files
        .iter()
        .filter(|file| file.status == "warn" || file.status == "block")
        .count() as u32;
    let blocked_files = files.iter().filter(|file| file.status == "block").count() as u32;
    let warning_files = files.iter().filter(|file| file.status == "warn").count() as u32;

    if (files_budget_exceeded || additions_budget_exceeded || deletions_budget_exceeded)
        && (risky_files > 0 || (!files_requiring_tests.is_empty() && tests_changed == 0))
    {
        findings.push(make_finding(
            "large_risky_diff",
            "Large diff with active risk signals",
            "block",
            "This patch is already over repo scope budgets and it also carries risk signals like sensitive paths or missing tests. Split or tighten it before moving forward.",
            files
                .iter()
                .filter(|file| file.status != "safe")
                .map(|file| file.path.clone())
                .take(6)
                .collect(),
        ));
    }

    let blocked_findings = findings
        .iter()
        .filter(|finding| finding.severity == "block")
        .count() as u32;
    let warning_findings = findings
        .iter()
        .filter(|finding| finding.severity == "warn")
        .count() as u32;

    let recommendation = if blocked_findings > 0 || blocked_files > 0 {
        "block"
    } else if warning_findings > 0 || warning_files > 0 {
        "warn"
    } else {
        "safe"
    };

    let risk_score = clamp_score(
        blocked_findings as usize * 28
            + warning_findings as usize * 12
            + blocked_files as usize * 10
            + warning_files as usize * 5
            + usize::from(!files_requiring_tests.is_empty() && tests_changed == 0) * 12
            + usize::from(generated_files > 0 && source_files_changed == 0) * 6,
    );

    let summary = match recommendation {
        "block" => format!(
            "Block this diff for now. TrustGate found {blocked_findings} blocking issues, {warning_findings} warnings, and {} risky files.",
            risky_files
        ),
        "warn" => format!(
            "Review closely before merge. TrustGate found {warning_findings} warnings across {risky_files} risky files."
        ),
        _ => format!(
            "This diff looks safe against the current repo rules. {} files changed with no active warnings.",
            files_changed
        ),
    };

    ReviewResult {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        repo: repo.to_string(),
        ai_source: if ai_source.trim().is_empty() {
            "unknown".into()
        } else {
            ai_source.trim().to_string()
        },
        recommendation: recommendation.into(),
        risk_score,
        summary,
        metrics: ReviewMetricSummary {
            files_changed,
            additions,
            deletions,
            tests_changed,
            risky_files,
            blocked_findings,
            warning_findings,
            generated_files,
            source_files_changed,
        },
        files,
        findings,
        rules,
        diff: diff.to_string(),
        source_kind: source_kind.into(),
        github: github_context,
        github_report: None,
    }
}

async fn fetch_pr_context(
    state: &AppState,
    repo: &str,
    pr_number: i64,
    trigger: &str,
    event: &str,
    action: &str,
) -> Result<(String, GitHubReviewContext), ApiError> {
    let token = github::github_token();
    let path = format!("/repos/{repo}/pulls/{pr_number}");
    let pr = github::get_json(&state.http, &path, token.as_deref())
        .await
        .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;
    let diff = github::get_text(
        &state.http,
        &path,
        "application/vnd.github.v3.diff",
        token.as_deref(),
    )
    .await
    .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;

    if diff.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_GATEWAY,
            "GitHub returned an empty PR diff.",
        ));
    }

    Ok((
        diff,
        GitHubReviewContext {
            repo: repo.to_string(),
            head_repo: pr["head"]["repo"]["full_name"]
                .as_str()
                .unwrap_or(repo)
                .to_string(),
            pr_number,
            pr_title: pr["title"].as_str().unwrap_or("").to_string(),
            pr_url: pr["html_url"].as_str().unwrap_or("").to_string(),
            head_sha: pr["head"]["sha"].as_str().unwrap_or("").to_string(),
            head_ref: pr["head"]["ref"].as_str().unwrap_or("").to_string(),
            base_ref: pr["base"]["ref"].as_str().unwrap_or("").to_string(),
            event: event.to_string(),
            action: action.to_string(),
            trigger: trigger.to_string(),
        },
    ))
}

fn verify_webhook_signature(headers: &HeaderMap, body: &[u8]) -> Result<(), ApiError> {
    let Some(secret) = github::webhook_secret() else {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Configure TRUST_GITHUB_WEBHOOK_SECRET before enabling TrustGate webhook ingestion.",
        ));
    };

    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "Missing X-Hub-Signature-256 header."))?;
    let signature_hex = signature
        .strip_prefix("sha256=")
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "Invalid webhook signature format."))?;
    let signature_bytes = hex::decode(signature_hex)
        .map_err(|_| api_error(StatusCode::UNAUTHORIZED, "Webhook signature was not valid hex."))?;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "Could not initialize webhook verification."))?;
    mac.update(body);
    mac.verify_slice(&signature_bytes)
        .map_err(|_| api_error(StatusCode::UNAUTHORIZED, "Webhook signature did not match."))?;

    Ok(())
}

pub async fn rule_packs() -> Json<Value> {
    Json(json!({ "packs": rule_packs_catalog() }))
}

pub async fn review(Json(body): Json<ReviewRequest>) -> Result<Json<ReviewResult>, ApiError> {
    let Some(repo) = db::normalize_repo_name(&body.repo) else {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "TrustGate expects repos in owner/repo format.",
        ));
    };

    if body.diff.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Paste a unified diff before running TrustGate.",
        ));
    }

    let rules = resolve_rules(&repo, body.rules)?;
    let review = review_diff(&repo, &body.diff, &body.ai_source, rules, "manual", None);
    db::save_review(&review)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(review))
}

pub async fn review_github_pr(
    State(state): State<AppState>,
    Json(body): Json<GitHubPrReviewRequest>,
) -> Result<Json<ReviewResult>, ApiError> {
    let Some(repo) = db::normalize_repo_name(&body.repo) else {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "TrustGate expects repos in owner/repo format.",
        ));
    };

    if body.pr_number <= 0 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "TrustGate needs a valid pull request number.",
        ));
    }

    let rules = resolve_rules(&repo, body.rules)?;
    let (diff, github_context) =
        fetch_pr_context(&state, &repo, body.pr_number, "manual_pr", "pull_request", "manual")
            .await?;
    let mut review = review_diff(
        &repo,
        &diff,
        &body.ai_source,
        rules,
        "github_pr",
        Some(github_context),
    );

    if body.publish_status {
        review.github_report = Some(github::publish_review_outcome(&state.http, &review).await);
    }

    db::save_review(&review)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(review))
}

pub async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    verify_webhook_signature(&headers, &body)?;

    let event = headers
        .get("X-GitHub-Event")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Webhook payload was not valid JSON."))?;

    if event != "pull_request" {
        return Ok(Json(json!({
            "triggered": false,
            "event": event,
            "reason": "TrustGate currently reacts only to pull_request webhooks.",
        })));
    }

    let action = payload["action"].as_str().unwrap_or("").to_string();
    if !matches!(
        action.as_str(),
        "opened" | "reopened" | "synchronize" | "ready_for_review"
    ) {
        return Ok(Json(json!({
            "triggered": false,
            "event": event,
            "action": action,
            "reason": "TrustGate ignores this pull_request action.",
        })));
    }

    let Some(repo) = payload["repository"]["full_name"]
        .as_str()
        .and_then(db::normalize_repo_name)
    else {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Webhook payload did not include a valid repository name.",
        ));
    };

    let pr_number = payload["number"]
        .as_i64()
        .or_else(|| payload["pull_request"]["number"].as_i64())
        .ok_or_else(|| {
            api_error(
                StatusCode::BAD_REQUEST,
                "Webhook payload did not include a pull request number.",
            )
        })?;

    let rules = resolve_rules(&repo, None)?;
    let (diff, github_context) =
        fetch_pr_context(&state, &repo, pr_number, "github_webhook", &event, &action).await?;
    let ai_source = payload["sender"]["login"]
        .as_str()
        .map(|login| format!("GitHub webhook via @{login}"))
        .unwrap_or_else(|| "GitHub webhook".into());

    let mut review = review_diff(
        &repo,
        &diff,
        &ai_source,
        rules,
        "github_webhook",
        Some(github_context),
    );
    review.github_report = Some(github::publish_review_outcome(&state.http, &review).await);

    db::save_review(&review)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(json!({
        "triggered": true,
        "event": event,
        "action": action,
        "recommendation": review.recommendation,
        "review": review,
    })))
}

pub async fn history() -> Result<Json<serde_json::Value>, ApiError> {
    let reviews = db::list_reviews()
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(json!({ "reviews": reviews })))
}

pub async fn history_detail(Path(id): Path<String>) -> Result<Json<ReviewResult>, ApiError> {
    let review = db::get_review(&id)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    match review {
        Some(review) => Ok(Json(review)),
        None => Err(api_error(StatusCode::NOT_FOUND, "TrustGate review not found.")),
    }
}

pub fn unique_repos(reviews: &[ReviewHistoryItem]) -> usize {
    reviews
        .iter()
        .map(|review| review.repo.clone())
        .collect::<BTreeSet<_>>()
        .len()
}
