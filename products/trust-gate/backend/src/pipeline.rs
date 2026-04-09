use std::collections::BTreeSet;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use glob::Pattern;
use patchhive_github_pr::verify_github_webhook_signature;
use serde_json::{json, Value};
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
    let lower = path.to_lowercase();
    lower.ends_with("package-lock.json")
        || lower.ends_with("pnpm-lock.yaml")
        || lower.ends_with("yarn.lock")
        || lower.ends_with("cargo.lock")
        || lower.contains("/dist/")
        || lower.contains("/build/")
        || lower.contains("/coverage/")
        || lower.contains("/generated/")
        || lower.ends_with(".snap")
        || lower.ends_with(".min.js")
        || lower.ends_with(".min.css")
        || lower.ends_with(".pb.go")
}

fn is_docs_only_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".md")
        || lower.starts_with("docs/")
        || lower.contains("/docs/")
        || lower.ends_with("changelog")
        || lower.ends_with("license")
}

fn path_policy_note(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();

    if lower.contains(".github/workflows/") {
        Some("Workflow edits can change CI behavior, release automation, or secret exposure.")
    } else if lower.contains("auth/") || lower.contains("permission") {
        Some("Auth and permission changes deserve extra scrutiny because small mistakes can broaden access.")
    } else if lower.contains("billing") {
        Some("Billing paths affect money movement and should be reviewed with policy and test coverage in mind.")
    } else if lower.contains("terraform/")
        || lower.contains("infra/")
        || lower.contains("dockerfile")
        || lower.contains("docker-compose")
    {
        Some("Infra/runtime changes can alter deployment, networking, or secret handling beyond the diff itself.")
    } else if lower.contains("migration") || lower.ends_with("schema.sql") {
        Some("Schema and migration changes can have irreversible data impact if they move forward too casually.")
    } else {
        None
    }
}

fn limit_examples(items: Vec<String>, limit: usize) -> Vec<String> {
    items.into_iter().take(limit).collect()
}

fn build_rule_packs() -> Vec<RulePack> {
    let mut app = RepoRuleSet::default();
    app.warn_paths.extend([
        "routes/".into(),
        "db/".into(),
        "api/".into(),
        "config/".into(),
    ]);
    app.require_test_for_paths.extend(["ui/".into(), "components/".into()]);
    app.max_files = 14;
    app.max_additions = 550;
    app.max_deletions = 300;
    app.notes = "Balanced app policy pack: strict on auth, workflows, and data boundaries while allowing normal feature work.".into();

    let mut library = RepoRuleSet::default();
    library.blocked_paths.extend(["examples/".into(), "benchmarks/".into()]);
    library.warn_paths.extend(["public_api".into(), "include/".into()]);
    library.require_test_for_paths
        .extend(["crates/".into(), "packages/".into()]);
    library.max_files = 10;
    library.max_additions = 320;
    library.max_deletions = 220;
    library.notes = "Library pack: tighter diff budgets and stronger test expectations around public surface changes.".into();

    let mut infra = RepoRuleSet::default();
    infra.blocked_paths.extend([
        "production/".into(),
        "modules/".into(),
        "environments/prod".into(),
    ]);
    infra.warn_paths.extend(["helm/".into(), "k8s/".into(), "deploy/".into()]);
    infra.require_test_for_paths = vec!["modules/".into(), "terraform/".into(), "scripts/".into()];
    infra.test_paths = vec!["tests/".into(), "plan/".into(), ".golden".into()];
    infra.max_files = 8;
    infra.max_additions = 260;
    infra.max_deletions = 160;
    infra.notes = "Infra pack: assumes runtime and deploy changes are high-risk, with low scope budgets and stronger escalation.".into();

    let mut agent_patch = RepoRuleSet::default();
    agent_patch.blocked_paths.extend([
        "prod/".into(),
        "release/".into(),
        "security/".into(),
    ]);
    agent_patch.warn_paths.extend([
        "src/".into(),
        "app/".into(),
        "server/".into(),
        "backend/".into(),
    ]);
    agent_patch.max_files = 6;
    agent_patch.max_additions = 220;
    agent_patch.max_deletions = 120;
    agent_patch.notes = "Agent-generated patch pack: strict scope budget designed for autonomous fixes that should stay narrow and test-backed.".into();

    vec![
        RulePack {
            id: "app".into(),
            label: "App".into(),
            description: "For product repos with UI, API, auth, and data layers that need balanced guardrails.".into(),
            rules: app,
        },
        RulePack {
            id: "library".into(),
            label: "Library".into(),
            description: "For SDKs and libraries where public surface changes and missing tests should be treated more strictly.".into(),
            rules: library,
        },
        RulePack {
            id: "infra".into(),
            label: "Infra".into(),
            description: "For deployment-heavy repos where workflow, runtime, and data-plane changes deserve aggressive escalation.".into(),
            rules: infra,
        },
        RulePack {
            id: "agent-patch".into(),
            label: "Agent Patch".into(),
            description: "For narrow autonomous fix repos where small, reversible, test-backed diffs are the standard.".into(),
            rules: agent_patch,
        },
    ]
}

fn resolve_rules(repo: &str, incoming: Option<RepoRuleSet>) -> Result<RepoRuleSet, ApiError> {
    let mut rules = if let Some(mut rules) = incoming {
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

fn normalize_ai_source(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.into()
    } else {
        value.trim().to_string()
    }
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
    let mut policy_hits = Vec::new();
    let mut generated_hits = Vec::new();
    let mut tests_changed = 0u32;
    let mut source_files_changed = 0u32;
    let mut generated_files = 0u32;
    let mut test_required_paths = Vec::new();
    let mut sensitive_code_changes = 0u32;

    for patch in patches {
        let generated = is_generated_path(&patch.path);
        let docs_only = is_docs_only_path(&patch.path);
        let path_policy = path_policy_note(&patch.path).unwrap_or("").to_string();
        let touches_test_path = !matching_patterns(&patch.path, &rules.test_paths).is_empty();
        let requires_tests = !generated
            && !docs_only
            && !touches_test_path
            && !matching_patterns(&patch.path, &rules.require_test_for_paths).is_empty();

        if touches_test_path {
            tests_changed += 1;
        }

        if generated {
            generated_files += 1;
            generated_hits.push(patch.path.clone());
        }

        if requires_tests {
            source_files_changed += 1;
            test_required_paths.push(patch.path.clone());
        }

        let mut status = "safe".to_string();
        let mut reasons = Vec::new();
        let mut matched_rules = Vec::new();

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
            sensitive_code_changes += 1;
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

        if !path_policy.is_empty() {
            policy_hits.push(format!("{} — {}", patch.path, path_policy));
        }

        let mut summary_parts = Vec::new();
        if !reasons.is_empty() {
            summary_parts.push(reasons.join(" "));
        }
        if !path_policy.is_empty() {
            summary_parts.push(path_policy.clone());
        }
        if generated {
            summary_parts.push(
                "Likely generated or lockfile output; review it together with the source of truth that produced it."
                    .into(),
            );
        }

        let summary = if summary_parts.is_empty() {
            "No immediate rule hits in this file.".to_string()
        } else {
            summary_parts.join(" ")
        };

        files.push(FileAssessment {
            path: patch.path,
            status,
            additions: patch.additions,
            deletions: patch.deletions,
            matched_rules,
            summary,
            generated,
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
            limit_examples(blocked_path_hits.clone(), 6),
        ));
    }

    if !warn_path_hits.is_empty() {
        findings.push(make_finding(
            "warn_paths",
            "Sensitive file paths",
            "warn",
            "The diff touches file areas that deserve extra scrutiny.",
            limit_examples(warn_path_hits, 6),
        ));
    }

    if !policy_hits.is_empty() {
        findings.push(make_finding(
            "path_policy",
            "Path-specific policy notes",
            if !blocked_path_hits.is_empty() { "block" } else { "warn" },
            "Some touched files sit on boundaries where even small edits can have outsized impact on trust, runtime, or data safety.",
            limit_examples(policy_hits, 6),
        ));
    }

    if !blocked_term_hits.is_empty() {
        findings.push(make_finding(
            "blocked_terms",
            "Blocked added content",
            "block",
            "The diff appears to add secret-like or explicitly banned content.",
            limit_examples(blocked_term_hits, 6),
        ));
    }

    if !suspicious_term_hits.is_empty() {
        findings.push(make_finding(
            "suspicious_terms",
            "Suspicious added content",
            "warn",
            "The diff adds lines that often correlate with fragile or risky changes.",
            limit_examples(suspicious_term_hits, 6),
        ));
    }

    if files_changed > rules.max_files {
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

    if additions > rules.max_additions {
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

    if deletions > rules.max_deletions {
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

    if generated_files > 0 {
        let severity = if generated_files >= 3 && source_files_changed == 0 {
            "block"
        } else {
            "warn"
        };
        let detail = if source_files_changed == 0 {
            "The diff changes generated artifacts or lockfiles without touching likely source files. Review the true source of change before moving forward."
        } else {
            "The diff includes generated artifacts or lockfiles. Review them alongside the code or configuration that produced them."
        };
        findings.push(make_finding(
            "generated_files",
            "Generated files changed",
            severity,
            detail,
            limit_examples(generated_hits, 6),
        ));
    }

    let missing_tests = !test_required_paths.is_empty() && tests_changed == 0;
    if missing_tests {
        let severity = if sensitive_code_changes > 0
            || source_files_changed > 2
            || additions > 140
            || deletions > 80
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
            limit_examples(test_required_paths.clone(), 6),
        ));
    }

    let risky_files = files
        .iter()
        .filter(|file| file.status == "warn" || file.status == "block")
        .count() as u32;

    if (files_changed > rules.max_files || additions > rules.max_additions || deletions > rules.max_deletions)
        && risky_files >= 3
    {
        findings.push(make_finding(
            "large_risky_diff",
            "Large diff with concentrated risk",
            "block",
            "This patch is both large and concentrated in sensitive areas. TrustGate should not treat it like a normal bounded AI patch.",
            vec![
                format!("{files_changed} files changed"),
                format!("{risky_files} risky files"),
                format!("{additions} additions / {deletions} deletions"),
            ],
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

    let recommendation = if blocked_findings > 0 {
        "block"
    } else if warning_findings > 0 {
        "warn"
    } else {
        "safe"
    };

    let risk_score = clamp_score(
        blocked_findings as usize * 34
            + warning_findings as usize * 11
            + risky_files as usize * 8
            + generated_files as usize * 4
            + source_files_changed as usize * 3
            + usize::from(missing_tests) * 12,
    );

    let summary = match recommendation {
        "block" => format!(
            "Block this diff for now. TrustGate found {blocked_findings} blocking issues and {warning_findings} warnings."
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
        ai_source: ai_source.to_string(),
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

async fn run_github_pr_review(
    client: &reqwest::Client,
    repo: String,
    pr_number: i64,
    ai_source: String,
    rules: Option<RepoRuleSet>,
    publish_status: bool,
    trigger: String,
    event: String,
    action: String,
) -> Result<ReviewResult, ApiError> {
    let Some(repo) = db::normalize_repo_name(&repo) else {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "TrustGate expects repos in owner/repo format.",
        ));
    };

    if pr_number <= 0 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "TrustGate expects a positive pull request number.",
        ));
    }

    let pr = github::fetch_pull_request(client, &repo, pr_number)
        .await
        .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;
    let diff = github::fetch_pull_request_diff(client, &repo, pr_number)
        .await
        .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;

    if diff.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_GATEWAY,
            "GitHub returned an empty pull request diff.",
        ));
    }

    let rules = resolve_rules(&repo, rules)?;
    let github_context = GitHubReviewContext {
        repo: repo.clone(),
        head_repo: if pr.head_repo.trim().is_empty() {
            repo.clone()
        } else {
            pr.head_repo.clone()
        },
        pr_number,
        pr_title: pr.title,
        pr_url: pr.html_url,
        head_sha: pr.head_sha,
        head_ref: pr.head_ref,
        base_ref: pr.base_ref,
        event,
        action,
        trigger,
    };

    let mut review = review_diff(
        &repo,
        &diff,
        &normalize_ai_source(&ai_source, "github-pr"),
        rules,
        "github_pr",
        Some(github_context),
    );

    review.github_report = Some(if publish_status {
        github::publish_review_outcome(client, &review).await
    } else {
        crate::models::GitHubReportOutcome {
            attempted: false,
            delivered: false,
            method: "none".into(),
            state: "skipped".into(),
            message: "GitHub status/check publishing was skipped for this run.".into(),
            details: Vec::new(),
            check_url: String::new(),
            status_url: String::new(),
            comment_url: String::new(),
            comment_mode: String::new(),
            report_markdown: String::new(),
        }
    });

    db::save_review(&review)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(review)
}

fn verify_webhook_signature(headers: &HeaderMap, body: &[u8]) -> Result<(), ApiError> {
    let Some(secret) = github::webhook_secret() else {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Configure TRUST_GITHUB_WEBHOOK_SECRET before enabling the TrustGate GitHub webhook.",
        ));
    };

    verify_github_webhook_signature(headers, body, &secret).map_err(|err| {
        let text = err.to_string();
        let status = if text.contains("Could not initialize") {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::UNAUTHORIZED
        };
        api_error(status, text)
    })
}

pub async fn rule_packs() -> Json<serde_json::Value> {
    Json(json!({ "packs": build_rule_packs() }))
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
    let review = review_diff(
        &repo,
        &body.diff,
        &normalize_ai_source(&body.ai_source, "manual"),
        rules,
        "manual",
        None,
    );
    db::save_review(&review)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(review))
}

pub async fn review_github_pr(
    State(state): State<AppState>,
    Json(body): Json<GitHubPrReviewRequest>,
) -> Result<Json<ReviewResult>, ApiError> {
    let review = run_github_pr_review(
        &state.http,
        body.repo,
        body.pr_number,
        body.ai_source,
        body.rules,
        body.publish_status,
        "manual_pr_lookup".into(),
        "pull_request".into(),
        "manual".into(),
    )
    .await?;

    Ok(Json(review))
}

pub async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_webhook_signature(&headers, &body)?;

    let event = headers
        .get("X-GitHub-Event")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| api_error(StatusCode::BAD_REQUEST, "Could not decode GitHub webhook payload."))?;

    if event != "pull_request" {
        return Ok(Json(json!({
            "triggered": false,
            "event": event,
            "reason": "TrustGate currently reviews pull_request webhooks only.",
        })));
    }

    let action = payload["action"].as_str().unwrap_or("").to_string();
    let supported = matches!(
        action.as_str(),
        "opened" | "reopened" | "synchronize" | "ready_for_review"
    );
    if !supported {
        return Ok(Json(json!({
            "triggered": false,
            "event": event,
            "action": action,
            "reason": "This pull_request action does not trigger an automatic TrustGate review.",
        })));
    }

    let repo = payload["repository"]["full_name"]
        .as_str()
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "Webhook payload was missing repository.full_name."))?
        .to_string();
    let pr_number = payload["pull_request"]["number"]
        .as_i64()
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, "Webhook payload was missing pull_request.number."))?;

    let review = run_github_pr_review(
        &state.http,
        repo,
        pr_number,
        "github-webhook".into(),
        None,
        true,
        "github_webhook".into(),
        event.clone(),
        action.clone(),
    )
    .await?;

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
