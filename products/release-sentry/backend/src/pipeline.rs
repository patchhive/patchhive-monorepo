use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use patchhive_product_core::{contract, startup::count_errors};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{
        auth_enabled, generate_and_save_key, generate_and_save_service_token,
        rotate_and_save_service_token, service_auth_enabled,
        service_token_generation_allowed_from_peer, service_token_rotation_allowed_from_peer,
        verify_token,
    },
    db, github,
    models::{
        HistoryItem, OverviewPayload, ReleaseCheck, ReleaseCheckRequest, ReleaseEvidenceLink,
        ReleaseReadinessMetrics, ReleaseReadinessResult,
    },
    state::AppState,
    STARTUP_CHECKS,
};

type ApiError = (StatusCode, Json<serde_json::Value>);
type JsonResult<T> = Result<Json<T>, ApiError>;

#[derive(serde::Deserialize)]
pub struct LoginBody {
    api_key: String,
}

pub async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "release-sentry",
        "ReleaseSentry",
        vec![contract::action(
            "check_github_release",
            "Check GitHub release readiness",
            "POST",
            "/check/github/release",
            "Gather release readiness evidence from GitHub releases, tags, changelog, blockers, and CI runs.",
            true,
        )],
        vec![
            contract::link("overview", "Overview", "/overview"),
            contract::link("history", "History", "/history"),
        ],
    ))
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    Json(contract::runs_from_history(
        "release-sentry",
        db::history(30),
    ))
}

pub async fn auth_status() -> Json<serde_json::Value> {
    Json(crate::auth::auth_status_payload())
}

pub async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth_enabled() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if !verify_token(&body.api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(
        json!({"ok": true, "auth_enabled": true, "auth_configured": true}),
    ))
}

pub async fn gen_key(
    headers: HeaderMap,
    peer: Option<patchhive_product_core::auth::ClientConnectInfo>,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if auth_enabled() {
        return Err(patchhive_product_core::auth::auth_already_configured_error());
    }
    let peer_addr = patchhive_product_core::auth::peer_addr_from_connect_info(peer);
    if !crate::auth::bootstrap_request_allowed_from_peer(&headers, peer_addr) {
        return Err(patchhive_product_core::auth::bootstrap_localhost_required_error());
    }
    let key = generate_and_save_key()
        .map_err(|err| patchhive_product_core::auth::key_generation_failed_error(&err))?;
    Ok(Json(
        json!({"api_key": key, "message": "Store this — it won't be shown again"}),
    ))
}

pub async fn gen_service_token(
    headers: HeaderMap,
    peer: Option<patchhive_product_core::auth::ClientConnectInfo>,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if service_auth_enabled() {
        return Err(patchhive_product_core::auth::service_auth_already_configured_error());
    }
    let peer_addr = patchhive_product_core::auth::peer_addr_from_connect_info(peer);
    if !service_token_generation_allowed_from_peer(&headers, peer_addr) {
        return Err(patchhive_product_core::auth::service_token_generation_forbidden_error());
    }
    let token = generate_and_save_service_token()
        .map_err(|err| patchhive_product_core::auth::service_token_generation_failed_error(&err))?;
    Ok(Json(json!({
        "service_token": token,
        "message": "Store this for HiveCore or other PatchHive service callers — it won't be shown again"
    })))
}

pub async fn rotate_service_token(
    headers: HeaderMap,
    peer: Option<patchhive_product_core::auth::ClientConnectInfo>,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if !service_auth_enabled() {
        return Err(patchhive_product_core::auth::service_auth_not_configured_error());
    }
    let peer_addr = patchhive_product_core::auth::peer_addr_from_connect_info(peer);
    if !service_token_rotation_allowed_from_peer(&headers, peer_addr) {
        return Err(patchhive_product_core::auth::service_token_rotation_forbidden_error());
    }
    let token = rotate_and_save_service_token()
        .map_err(|err| patchhive_product_core::auth::service_token_rotation_failed_error(&err))?;
    Ok(Json(json!({
        "service_token": token,
        "message": "Store this replacement service token for HiveCore or other PatchHive service callers — it won't be shown again"
    })))
}

pub async fn health() -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();
    let counts = db::overview_counts();

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "ReleaseSentry by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "github_ready": github::github_token_configured(),
        "run_count": counts.runs,
        "repo_count": counts.repos,
        "ready_count": counts.ready,
        "watch_count": counts.watch,
        "hold_count": counts.hold,
        "mode": "release-readiness",
    }))
}

pub async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

pub async fn overview() -> Json<OverviewPayload> {
    Json(db::overview())
}

pub async fn history() -> Json<Vec<HistoryItem>> {
    Json(db::history(30))
}

pub async fn history_detail(Path(id): Path<String>) -> JsonResult<ReleaseReadinessResult> {
    db::get_run(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "ReleaseSentry run not found"))
}

pub async fn check_github_release(
    State(state): State<AppState>,
    Json(request): Json<ReleaseCheckRequest>,
) -> JsonResult<ReleaseReadinessResult> {
    let repo = request.repo.trim();
    if !valid_repo(repo) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Repository must be in owner/name format.",
        ));
    }

    let result = build_release_readiness(&state, request)
        .await
        .map_err(upstream_error)?;
    db::save_run(&result)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(result))
}

async fn build_release_readiness(
    state: &AppState,
    request: ReleaseCheckRequest,
) -> anyhow::Result<ReleaseReadinessResult> {
    let repo = request.repo.trim().to_string();
    let repository = github::fetch_repository(&state.http, &repo).await?;
    let branch = normalize_branch(&request.branch, &repository.default_branch);
    let target_version = request.target_version.trim().to_string();
    let target_tag = normalize_target_tag(&request.target_tag, &target_version);
    let mut checks = Vec::new();
    let mut warnings = Vec::new();

    checks.push(check_repository(&repository, &branch));

    let releases = match github::fetch_releases(&state.http, &repo, 20).await {
        Ok(value) => value,
        Err(err) => {
            warnings.push(format!("Could not read GitHub releases for {repo}: {err}"));
            Vec::new()
        }
    };
    checks.push(check_releases(&releases, &target_tag));

    let tags = match github::fetch_tags(&state.http, &repo, 50).await {
        Ok(value) => value,
        Err(err) => {
            warnings.push(format!("Could not read GitHub tags for {repo}: {err}"));
            Vec::new()
        }
    };
    checks.push(check_tags(&tags, &target_tag));

    let runs = match github::fetch_workflow_runs(
        &state.http,
        &repo,
        Some(branch.as_str()),
        request.workflow_run_limit.clamp(5, 100),
    )
    .await
    {
        Ok(value) => value,
        Err(err) => {
            warnings.push(format!(
                "Could not read GitHub Actions runs for {repo}: {err}"
            ));
            Vec::new()
        }
    };
    checks.push(check_workflows(&runs, &branch));

    let blocker_labels = normalize_blocker_labels(&request.blocker_labels);
    let issues = match github::fetch_open_issues(&state.http, &repo, 100).await {
        Ok(value) => value,
        Err(err) => {
            warnings.push(format!("Could not read open issues for {repo}: {err}"));
            Vec::new()
        }
    };
    checks.push(check_blockers(&issues, &blocker_labels));

    let changelog_path = request.changelog_path.trim();
    if changelog_path.is_empty() {
        checks.push(ReleaseCheck {
            key: "changelog".into(),
            label: "Changelog".into(),
            status: "warn".into(),
            detail:
                "No changelog path was provided, so ReleaseSentry could not verify release notes."
                    .into(),
            ..ReleaseCheck::default()
        });
    } else {
        checks.push(
            check_changelog(
                &state.http,
                &repo,
                &branch,
                changelog_path,
                &target_version,
                &target_tag,
            )
            .await,
        );
    }

    checks.push(
        check_release_surface(&state.http, &repo, &branch)
            .await
            .unwrap_or_else(|err| ReleaseCheck {
                key: "release-surface".into(),
                label: "Release Surface".into(),
                status: "warn".into(),
                detail: format!("Could not inspect common release surface files: {err}"),
                ..ReleaseCheck::default()
            }),
    );

    let metrics = build_metrics(
        &checks,
        &runs,
        &issues,
        &blocker_labels,
        tags.len(),
        releases.len(),
    );
    let decision = decision_for_metrics(&metrics);
    let score = score_for_metrics(&metrics);
    let summary = build_summary(&repo, &branch, &decision, &metrics, &target_tag);
    let now = Utc::now().to_rfc3339();

    Ok(ReleaseReadinessResult {
        id: Uuid::new_v4().to_string(),
        created_at: now.clone(),
        updated_at: now,
        repo,
        branch,
        target_version,
        target_tag,
        status: decision.clone(),
        decision: decision.clone(),
        score,
        title: format!("{} release readiness", repository.full_name),
        summary,
        metrics,
        checks,
        warnings,
    })
}

fn check_repository(repository: &github::GitHubRepositoryDetail, branch: &str) -> ReleaseCheck {
    let mut evidence = vec![format!("Default branch: {}", repository.default_branch)];
    if let Some(pushed_at) = &repository.pushed_at {
        evidence.push(format!("Last push: {pushed_at}"));
    }
    let status = if repository.archived || repository.disabled {
        "block"
    } else if repository.default_branch != branch && !branch.is_empty() {
        "warn"
    } else {
        "pass"
    };
    let detail = match status {
        "block" => "Repository is archived or disabled, so release automation should hold.".into(),
        "warn" => format!(
            "Requested branch `{branch}` differs from the repository default branch `{}`.",
            repository.default_branch
        ),
        _ => "Repository is reachable and active.".into(),
    };
    ReleaseCheck {
        key: "repository".into(),
        label: "Repository".into(),
        status: status.into(),
        detail,
        evidence,
        links: vec![ReleaseEvidenceLink {
            label: "Repository".into(),
            url: repository.html_url.clone(),
        }],
    }
}

fn check_releases(releases: &[github::GitHubRelease], target_tag: &str) -> ReleaseCheck {
    if releases.is_empty() {
        return ReleaseCheck {
            key: "release-history".into(),
            label: "Release History".into(),
            status: "warn".into(),
            detail: "No GitHub releases were found. This may be the first release, but the ship call needs extra human attention.".into(),
            ..ReleaseCheck::default()
        };
    }

    if !target_tag.is_empty() {
        if let Some(release) = releases
            .iter()
            .find(|release| release.tag_name == target_tag)
        {
            return ReleaseCheck {
                key: "release-history".into(),
                label: "Release History".into(),
                status: if release.draft { "warn" } else { "pass" }.into(),
                detail: if release.draft {
                    format!("Target release `{target_tag}` exists, but it is still a draft.")
                } else {
                    format!("Target release `{target_tag}` exists in GitHub releases.")
                },
                evidence: release_evidence(release),
                links: release_link(release),
            };
        }
    }

    let latest = &releases[0];
    ReleaseCheck {
        key: "release-history".into(),
        label: "Release History".into(),
        status: if target_tag.is_empty() {
            "pass"
        } else {
            "warn"
        }
        .into(),
        detail: if target_tag.is_empty() {
            format!("Latest release is `{}`.", latest.tag_name)
        } else {
            format!(
                "Target release `{target_tag}` is not published yet. Latest release is `{}`.",
                latest.tag_name
            )
        },
        evidence: release_evidence(latest),
        links: release_link(latest),
    }
}

fn check_tags(tags: &[github::GitHubTag], target_tag: &str) -> ReleaseCheck {
    if tags.is_empty() {
        return ReleaseCheck {
            key: "tags".into(),
            label: "Tags".into(),
            status: "warn".into(),
            detail: "No tags were found in GitHub's recent tag list.".into(),
            ..ReleaseCheck::default()
        };
    }

    let evidence = tags
        .iter()
        .take(5)
        .map(|tag| format!("{} · {}", tag.name, short_sha(&tag.commit.sha)))
        .collect::<Vec<_>>();
    let status = if target_tag.is_empty() || tags.iter().any(|tag| tag.name == target_tag) {
        "pass"
    } else {
        "warn"
    };
    let detail = if target_tag.is_empty() {
        format!(
            "ReleaseSentry saw {} recent tag{}.",
            tags.len(),
            plural(tags.len() as u32)
        )
    } else if status == "pass" {
        format!("Target tag `{target_tag}` exists.")
    } else {
        format!("Target tag `{target_tag}` was not found in the recent tag list.")
    };
    ReleaseCheck {
        key: "tags".into(),
        label: "Tags".into(),
        status: status.into(),
        detail,
        evidence,
        ..ReleaseCheck::default()
    }
}

fn check_workflows(runs: &[github::GitHubActionsWorkflowRun], branch: &str) -> ReleaseCheck {
    if runs.is_empty() {
        return ReleaseCheck {
            key: "ci-health".into(),
            label: "CI Health".into(),
            status: "warn".into(),
            detail: format!("No recent GitHub Actions runs were found for `{branch}`."),
            ..ReleaseCheck::default()
        };
    }

    let failures = runs
        .iter()
        .filter(|run| failing_conclusion(&run.conclusion))
        .count();
    let pending = runs
        .iter()
        .filter(|run| run.conclusion.trim().is_empty())
        .count();
    let successes = runs
        .iter()
        .filter(|run| run.conclusion == "success")
        .count();
    let status = if failures > 0 {
        "block"
    } else if pending > 0 || successes == 0 {
        "warn"
    } else {
        "pass"
    };
    let evidence = runs
        .iter()
        .take(6)
        .map(|run| {
            format!(
                "{} #{} · {}",
                if run.name.is_empty() {
                    "workflow"
                } else {
                    &run.name
                },
                run.run_number,
                if run.conclusion.is_empty() {
                    "pending"
                } else {
                    &run.conclusion
                }
            )
        })
        .collect::<Vec<_>>();
    ReleaseCheck {
        key: "ci-health".into(),
        label: "CI Health".into(),
        status: status.into(),
        detail: format!(
            "{} successful run{}, {} failed run{}, {} pending run{} across {} recent workflow run{} on `{branch}`.",
            successes,
            plural(successes as u32),
            failures,
            plural(failures as u32),
            pending,
            plural(pending as u32),
            runs.len(),
            plural(runs.len() as u32)
        ),
        evidence,
        links: runs
            .first()
            .map(|run| ReleaseEvidenceLink {
                label: "Latest workflow run".into(),
                url: run.html_url.clone(),
            })
            .into_iter()
            .collect(),
    }
}

fn check_blockers(issues: &[github::GitHubIssue], blocker_labels: &[String]) -> ReleaseCheck {
    let blockers = issues
        .iter()
        .filter(|issue| issue.pull_request.is_none())
        .filter(|issue| {
            issue.labels.iter().any(|label| {
                let name = label.name.to_ascii_lowercase();
                blocker_labels.iter().any(|needle| name.contains(needle))
            })
        })
        .collect::<Vec<_>>();

    let evidence = blockers
        .iter()
        .take(5)
        .map(|issue| format!("#{} · {}", issue.number, issue.title))
        .collect::<Vec<_>>();
    ReleaseCheck {
        key: "release-blockers".into(),
        label: "Release Blockers".into(),
        status: if blockers.is_empty() { "pass" } else { "block" }.into(),
        detail: if blockers.is_empty() {
            "No open release-blocker issues were found with the configured labels.".into()
        } else {
            format!(
                "{} open issue{} matched the release blocker label set.",
                blockers.len(),
                plural(blockers.len() as u32)
            )
        },
        evidence,
        links: blockers
            .first()
            .map(|issue| ReleaseEvidenceLink {
                label: "Top blocker".into(),
                url: issue.html_url.clone(),
            })
            .into_iter()
            .collect(),
    }
}

async fn check_changelog(
    client: &reqwest::Client,
    repo: &str,
    branch: &str,
    path: &str,
    target_version: &str,
    target_tag: &str,
) -> ReleaseCheck {
    let file = match github::fetch_content_text(client, repo, path, branch).await {
        Ok(value) => value,
        Err(err) => {
            return ReleaseCheck {
                key: "changelog".into(),
                label: "Changelog".into(),
                status: "warn".into(),
                detail: format!("Could not read `{path}` on `{branch}`: {err}"),
                ..ReleaseCheck::default()
            }
        }
    };
    let text = github::decode_content(&file).unwrap_or_default();
    let needles = [target_version.trim(), target_tag.trim()]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let mentions_target = needles.iter().any(|needle| text.contains(needle));
    let status = if needles.is_empty() || mentions_target {
        "pass"
    } else {
        "warn"
    };
    ReleaseCheck {
        key: "changelog".into(),
        label: "Changelog".into(),
        status: status.into(),
        detail: if needles.is_empty() {
            format!("`{path}` exists. No target version was provided, so ReleaseSentry only checked for release-note presence.")
        } else if mentions_target {
            format!("`{path}` mentions the target version or tag.")
        } else {
            format!("`{path}` exists, but it does not mention the target version/tag yet.")
        },
        evidence: vec![format!("{} bytes decoded from `{path}`", text.len())],
        links: vec![ReleaseEvidenceLink {
            label: path.into(),
            url: file.html_url,
        }],
    }
}

async fn check_release_surface(
    client: &reqwest::Client,
    repo: &str,
    branch: &str,
) -> anyhow::Result<ReleaseCheck> {
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for path in [
        "Cargo.toml",
        "package.json",
        "docker-compose.yml",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
    ] {
        match github::fetch_content_text(client, repo, path, branch).await {
            Ok(_) => found.push(path.to_string()),
            Err(_) => missing.push(path.to_string()),
        }
    }

    let has_manifest = found
        .iter()
        .any(|path| path == "Cargo.toml" || path == "package.json");
    let has_release_path = found.iter().any(|path| {
        path == "docker-compose.yml"
            || path == ".github/workflows/ci.yml"
            || path == ".github/workflows/release.yml"
    });
    let status = if has_manifest && has_release_path {
        "pass"
    } else {
        "warn"
    };
    Ok(ReleaseCheck {
        key: "release-surface".into(),
        label: "Release Surface".into(),
        status: status.into(),
        detail: if status == "pass" {
            "Common package and release surface files are present.".into()
        } else {
            "ReleaseSentry could not see both package metadata and a release/CI surface.".into()
        },
        evidence: vec![
            format!("Found: {}", fallback_join(&found, "none")),
            format!("Missing: {}", fallback_join(&missing, "none")),
        ],
        ..ReleaseCheck::default()
    })
}

fn build_metrics(
    checks: &[ReleaseCheck],
    runs: &[github::GitHubActionsWorkflowRun],
    issues: &[github::GitHubIssue],
    blocker_labels: &[String],
    tags_seen: usize,
    releases_seen: usize,
) -> ReleaseReadinessMetrics {
    let mut metrics = ReleaseReadinessMetrics {
        checks: checks.len() as u32,
        workflow_runs: runs.len() as u32,
        tags_seen: tags_seen as u32,
        releases_seen: releases_seen as u32,
        ..ReleaseReadinessMetrics::default()
    };

    for check in checks {
        match check.status.as_str() {
            "block" => metrics.blocked += 1,
            "warn" => metrics.warned += 1,
            "pass" => metrics.passed += 1,
            _ => {}
        }
    }
    for run in runs {
        if run.conclusion == "success" {
            metrics.workflow_successes += 1;
        } else if run.conclusion.trim().is_empty() {
            metrics.workflow_pending += 1;
        } else if failing_conclusion(&run.conclusion) {
            metrics.workflow_failures += 1;
        }
    }
    metrics.release_blockers = issues
        .iter()
        .filter(|issue| issue.pull_request.is_none())
        .filter(|issue| {
            issue.labels.iter().any(|label| {
                let name = label.name.to_ascii_lowercase();
                blocker_labels.iter().any(|needle| name.contains(needle))
            })
        })
        .count() as u32;
    metrics
}

fn decision_for_metrics(metrics: &ReleaseReadinessMetrics) -> String {
    if metrics.blocked > 0 {
        "hold".into()
    } else if metrics.warned > 0 {
        "watch".into()
    } else {
        "ready".into()
    }
}

fn score_for_metrics(metrics: &ReleaseReadinessMetrics) -> u32 {
    100u32
        .saturating_sub(metrics.blocked.saturating_mul(25))
        .saturating_sub(metrics.warned.saturating_mul(8))
        .max(1)
}

fn build_summary(
    repo: &str,
    branch: &str,
    decision: &str,
    metrics: &ReleaseReadinessMetrics,
    target_tag: &str,
) -> String {
    let target = if target_tag.is_empty() {
        "the next release".into()
    } else {
        format!("`{target_tag}`")
    };
    match decision {
        "ready" => format!(
            "ReleaseSentry says {repo} is ready to ship {target}: {} checks passed on `{branch}`.",
            metrics.passed
        ),
        "hold" => format!(
            "ReleaseSentry says hold {target} for {repo}: {} blocking gate check{} need attention.",
            metrics.blocked,
            plural(metrics.blocked)
        ),
        _ => format!(
            "ReleaseSentry says watch {target} for {repo}: {} passed, {} warned, 0 blocked.",
            metrics.passed, metrics.warned
        ),
    }
}

fn normalize_branch(requested: &str, default_branch: &str) -> String {
    let requested = requested.trim();
    if requested.is_empty() {
        default_branch.trim().to_string()
    } else {
        requested.to_string()
    }
}

fn normalize_target_tag(requested: &str, version: &str) -> String {
    let requested = requested.trim();
    if !requested.is_empty() {
        return requested.into();
    }
    let version = version.trim();
    if version.is_empty() {
        String::new()
    } else if version.starts_with('v') {
        version.into()
    } else {
        format!("v{version}")
    }
}

fn normalize_blocker_labels(labels: &[String]) -> Vec<String> {
    let mut labels = labels
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if labels.is_empty() {
        labels = vec![
            "release-blocker".into(),
            "blocker".into(),
            "critical".into(),
            "regression".into(),
        ];
    }
    labels
}

fn release_evidence(release: &github::GitHubRelease) -> Vec<String> {
    vec![
        format!("tag: {}", release.tag_name),
        format!("draft: {}", release.draft),
        format!("prerelease: {}", release.prerelease),
        format!(
            "published: {}",
            release.published_at.as_deref().unwrap_or("not published")
        ),
    ]
}

fn release_link(release: &github::GitHubRelease) -> Vec<ReleaseEvidenceLink> {
    if release.html_url.is_empty() {
        Vec::new()
    } else {
        vec![ReleaseEvidenceLink {
            label: "Release".into(),
            url: release.html_url.clone(),
        }]
    }
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

fn upstream_error(error: anyhow::Error) -> ApiError {
    api_error(StatusCode::BAD_GATEWAY, error.to_string())
}

fn failing_conclusion(conclusion: &str) -> bool {
    matches!(
        conclusion,
        "failure" | "cancelled" | "timed_out" | "action_required" | "startup_failure"
    )
}

fn short_sha(value: &str) -> String {
    value.chars().take(7).collect::<String>()
}

fn fallback_join(values: &[String], fallback: &str) -> String {
    if values.is_empty() {
        fallback.into()
    } else {
        values.join(", ")
    }
}

fn plural(value: u32) -> &'static str {
    if value == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::{check_workflows, decision_for_metrics, normalize_target_tag, score_for_metrics};
    use crate::models::ReleaseReadinessMetrics;
    use patchhive_github_data::models::GitHubActionsWorkflowRun;

    #[test]
    fn derives_target_tag_from_version() {
        assert_eq!(normalize_target_tag("", "1.2.3"), "v1.2.3");
        assert_eq!(normalize_target_tag("", "v2.0.0"), "v2.0.0");
        assert_eq!(normalize_target_tag("release-7", "1.2.3"), "release-7");
    }

    #[test]
    fn holds_when_any_check_blocks() {
        let metrics = ReleaseReadinessMetrics {
            blocked: 1,
            warned: 4,
            ..ReleaseReadinessMetrics::default()
        };
        assert_eq!(decision_for_metrics(&metrics), "hold");
        assert!(score_for_metrics(&metrics) < 75);
    }

    #[test]
    fn watches_when_only_warnings_remain() {
        let metrics = ReleaseReadinessMetrics {
            passed: 5,
            warned: 1,
            ..ReleaseReadinessMetrics::default()
        };
        assert_eq!(decision_for_metrics(&metrics), "watch");
        assert_eq!(score_for_metrics(&metrics), 92);
    }

    #[test]
    fn ci_detail_uses_readable_plural_words() {
        let runs = vec![
            GitHubActionsWorkflowRun {
                conclusion: "success".into(),
                ..GitHubActionsWorkflowRun::default()
            },
            GitHubActionsWorkflowRun {
                conclusion: "success".into(),
                ..GitHubActionsWorkflowRun::default()
            },
            GitHubActionsWorkflowRun {
                conclusion: "failure".into(),
                ..GitHubActionsWorkflowRun::default()
            },
        ];

        let check = check_workflows(&runs, "main");

        assert!(check.detail.contains("2 successful runs"));
        assert!(check.detail.contains("1 failed run"));
        assert!(!check.detail.contains("successs"));
    }
}
