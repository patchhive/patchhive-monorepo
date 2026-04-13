use std::collections::{BTreeMap, HashMap};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use patchhive_product_core::startup::count_errors;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{auth_enabled, generate_and_save_key, verify_token},
    db, github,
    github::{GitHubWorkflowJob, GitHubWorkflowRun},
    models::{
        compute_trend, FlakeMetrics, FlakeScanResult, FlakeSignal, HistoryItem,
        OverviewPayload, ScanRequest,
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

#[derive(Default)]
struct SignalBucket {
    kind: String,
    workflow_name: String,
    job_name: String,
    step_name: String,
    failure_count: u32,
    success_count: u32,
    rerun_hits: u32,
    fail_envs: BTreeMap<String, u32>,
    success_envs: BTreeMap<String, u32>,
    evidence: Vec<String>,
}

pub async fn auth_status() -> Json<serde_json::Value> {
    Json(json!({"auth_enabled": auth_enabled()}))
}

pub async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !verify_token(&body.api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(json!({"ok": true, "auth_enabled": true})))
}

pub async fn gen_key() -> Result<Json<serde_json::Value>, StatusCode> {
    if auth_enabled() {
        return Err(StatusCode::FORBIDDEN);
    }
    let key = generate_and_save_key();
    Ok(Json(json!({"api_key": key, "message": "Store this — it won't be shown again"})))
}

pub async fn health() -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let counts = db::overview_counts();

    Json(json!({
        "status": if errors > 0 { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "FlakeSting by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_path": db::db_path(),
        "github_ready": github::github_token_configured(),
        "scan_count": counts.scans,
        "repo_count": counts.repos,
        "flaky_signal_count": counts.flaky_signals,
        "quarantine_candidate_count": counts.quarantine_candidates,
        "mode": "github-actions-flake-detection",
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

pub async fn history_detail(Path(id): Path<String>) -> JsonResult<FlakeScanResult> {
    db::get_scan(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "FlakeSting scan not found"))
}

pub async fn scan_github_actions(
    State(state): State<AppState>,
    Json(request): Json<ScanRequest>,
) -> JsonResult<FlakeScanResult> {
    let repo = request.repo.trim();
    if !valid_repo(repo) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Repository must be in owner/name format.",
        ));
    }

    let lookback_runs = request.lookback_runs.clamp(5, 40);
    let branch = request.branch.trim().to_string();
    let workflow_name = request.workflow_name.trim().to_string();

    let runs = github::fetch_workflow_runs(
        &state.http,
        repo,
        if branch.is_empty() { None } else { Some(branch.as_str()) },
        lookback_runs,
    )
    .await
    .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;

    let result = build_scan_result(
        &state,
        repo.to_string(),
        branch,
        workflow_name,
        lookback_runs,
        runs,
    )
    .await
    .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;

    db::save_scan(&result)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(result))
}

fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

fn is_failure(conclusion: &str) -> bool {
    matches!(
        conclusion,
        "failure" | "timed_out" | "cancelled" | "action_required" | "startup_failure" | "stale"
    )
}

fn is_success(conclusion: &str) -> bool {
    conclusion == "success"
}

fn is_testish(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "test",
        "spec",
        "integration",
        "unit",
        "e2e",
        "pytest",
        "cargo test",
        "jest",
        "vitest",
        "cypress",
        "playwright",
        "go test",
        "rspec",
        "mvn test",
        "gradle test",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn runner_label(job: &GitHubWorkflowJob) -> String {
    if !job.runner_name.trim().is_empty() {
        job.runner_name.clone()
    } else if !job.labels.is_empty() {
        job.labels.join(", ")
    } else {
        "unknown-runner".into()
    }
}

fn push_evidence(items: &mut Vec<String>, value: String) {
    if value.trim().is_empty() || items.iter().any(|item| item == &value) {
        return;
    }
    if items.len() < 6 {
        items.push(value);
    }
}

fn record_bucket(
    buckets: &mut HashMap<String, SignalBucket>,
    kind: &str,
    workflow_name: &str,
    job_name: &str,
    step_name: &str,
    conclusion: &str,
    run: &GitHubWorkflowRun,
    runner: &str,
    evidence_url: &str,
) {
    let key = format!("{kind}:{workflow_name}:{job_name}:{step_name}");
    let bucket = buckets.entry(key).or_default();
    if bucket.kind.is_empty() {
        bucket.kind = kind.into();
        bucket.workflow_name = workflow_name.into();
        bucket.job_name = job_name.into();
        bucket.step_name = step_name.into();
    }

    if is_failure(conclusion) {
        bucket.failure_count += 1;
        *bucket.fail_envs.entry(runner.into()).or_default() += 1;
    } else if is_success(conclusion) {
        bucket.success_count += 1;
        *bucket.success_envs.entry(runner.into()).or_default() += 1;
    } else {
        return;
    }

    if run.run_attempt > 1 {
        bucket.rerun_hits += 1;
    }

    let line = format!(
        "run #{} attempt {} → {} on {}{}{}",
        run.run_number,
        run.run_attempt.max(1),
        if conclusion.trim().is_empty() {
            "unknown"
        } else {
            conclusion
        },
        runner,
        if run.html_url.trim().is_empty() { "" } else { " · " },
        evidence_url,
    );
    push_evidence(&mut bucket.evidence, line);
}

fn strongest_env(map: &BTreeMap<String, u32>) -> Option<(String, u32)> {
    map.iter()
        .max_by(|left, right| left.1.cmp(right.1).then_with(|| left.0.cmp(right.0)))
        .map(|(env, count)| (env.clone(), *count))
}

fn environment_hints(bucket: &SignalBucket) -> Vec<String> {
    let mut hints = Vec::new();
    if let Some((fail_env, fail_count)) = strongest_env(&bucket.fail_envs) {
        let success_count = bucket.success_envs.get(&fail_env).copied().unwrap_or(0);
        if fail_count >= 2 && success_count == 0 {
            hints.push(format!(
                "Failures are clustering on `{fail_env}` while passes are showing up elsewhere."
            ));
        }
    }
    if bucket.rerun_hits > 0 {
        hints.push(format!(
            "{} signal hit{} came from rerun attempts, which is a classic flake smell.",
            bucket.rerun_hits,
            if bucket.rerun_hits == 1 { "" } else { "s" }
        ));
    }
    hints
}

fn signal_score(bucket: &SignalBucket) -> u32 {
    let overlap = bucket.failure_count.min(bucket.success_count);
    let mut score = overlap * 22 + bucket.failure_count * 16 + bucket.rerun_hits * 10;
    if environment_hints(bucket).iter().any(|hint| hint.contains("clustering")) {
        score += 12;
    }
    score.min(100)
}

fn build_signal(bucket: SignalBucket) -> Option<FlakeSignal> {
    if bucket.failure_count == 0 || bucket.success_count == 0 {
        return None;
    }
    let total = bucket.failure_count + bucket.success_count;
    if total < 2 {
        return None;
    }

    let status = if bucket.failure_count >= 2 && bucket.success_count >= 2 {
        "quarantine"
    } else {
        "suspect"
    };
    let score = signal_score(&bucket);
    let environment_hints = environment_hints(&bucket);

    let target_label = if bucket.step_name.trim().is_empty() {
        format!("job `{}`", bucket.job_name)
    } else {
        format!("step `{}` inside `{}`", bucket.step_name, bucket.job_name)
    };
    let summary = format!(
        "{} in workflow `{}` failed {} time{} and passed {} time{} across recent runs.",
        target_label,
        bucket.workflow_name,
        bucket.failure_count,
        if bucket.failure_count == 1 { "" } else { "s" },
        bucket.success_count,
        if bucket.success_count == 1 { "" } else { "s" },
    );

    Some(FlakeSignal {
        key: format!(
            "{}:{}:{}:{}",
            bucket.kind, bucket.workflow_name, bucket.job_name, bucket.step_name
        ),
        kind: bucket.kind,
        status: status.into(),
        score,
        workflow_name: bucket.workflow_name,
        job_name: bucket.job_name,
        step_name: bucket.step_name,
        summary,
        failure_count: bucket.failure_count,
        success_count: bucket.success_count,
        rerun_hits: bucket.rerun_hits,
        environment_hints,
        evidence: bucket.evidence,
    })
}

async fn build_scan_result(
    state: &AppState,
    repo: String,
    branch: String,
    workflow_filter: String,
    _lookback_runs: u32,
    runs: Vec<GitHubWorkflowRun>,
) -> anyhow::Result<FlakeScanResult> {
    let normalized_filter = workflow_filter.to_ascii_lowercase();
    let filtered_runs = runs
        .into_iter()
        .filter(|run| {
            run.id > 0
                && !run.conclusion.trim().is_empty()
                && (normalized_filter.is_empty()
                    || run.name.to_ascii_lowercase().contains(&normalized_filter))
        })
        .collect::<Vec<_>>();

    let mut buckets = HashMap::<String, SignalBucket>::new();
    let mut completed_runs = 0u32;
    let mut successful_runs = 0u32;
    let mut failed_runs = 0u32;
    let mut rerun_like_runs = 0u32;

    for run in &filtered_runs {
        completed_runs += 1;
        if is_success(&run.conclusion) {
            successful_runs += 1;
        } else if is_failure(&run.conclusion) {
            failed_runs += 1;
        }
        if run.run_attempt > 1 {
            rerun_like_runs += 1;
        }

        let jobs = github::fetch_workflow_jobs(&state.http, &repo, run.id).await?;
        for job in jobs {
            let runner = runner_label(&job);
            let job_testish = is_testish(&job.name) || is_testish(&run.name);
            let mut recorded_step = false;

            for step in &job.steps {
                if !(job_testish || is_testish(&step.name)) {
                    continue;
                }
                if !(is_success(&step.conclusion) || is_failure(&step.conclusion)) {
                    continue;
                }
                recorded_step = true;
                record_bucket(
                    &mut buckets,
                    "step",
                    &run.name,
                    &job.name,
                    &step.name,
                    &step.conclusion,
                    run,
                    &runner,
                    &job.html_url,
                );
            }

            if !recorded_step && job_testish && (is_success(&job.conclusion) || is_failure(&job.conclusion)) {
                record_bucket(
                    &mut buckets,
                    "job",
                    &run.name,
                    &job.name,
                    "",
                    &job.conclusion,
                    run,
                    &runner,
                    &job.html_url,
                );
            }
        }
    }

    let mut signals = buckets
        .into_values()
        .filter_map(build_signal)
        .collect::<Vec<_>>();
    signals.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.failure_count.cmp(&left.failure_count))
            .then_with(|| right.workflow_name.cmp(&left.workflow_name))
            .then_with(|| right.job_name.cmp(&left.job_name))
            .then_with(|| right.step_name.cmp(&left.step_name))
    });

    let quarantine_candidates = signals
        .iter()
        .filter(|signal| signal.status == "quarantine")
        .count() as u32;
    let metrics = FlakeMetrics {
        workflow_runs: filtered_runs.len() as u32,
        completed_runs,
        successful_runs,
        failed_runs,
        rerun_like_runs,
        flaky_signals: signals.len() as u32,
        quarantine_candidates,
    };
    let summary = if signals.is_empty() {
        format!(
            "FlakeSting did not find fail/pass swings in the last {} matching workflow run{} for `{}`.",
            metrics.workflow_runs,
            if metrics.workflow_runs == 1 { "" } else { "s" },
            repo
        )
    } else {
        let top = &signals[0];
        format!(
            "FlakeSting found {} flaky signal{} across the last {} matching workflow run{}. Strongest suspect: {}",
            signals.len(),
            if signals.len() == 1 { "" } else { "s" },
            metrics.workflow_runs,
            if metrics.workflow_runs == 1 { "" } else { "s" },
            top.summary
        )
    };

    let branch_value = if branch.trim().is_empty() {
        filtered_runs
            .first()
            .map(|run| run.head_branch.clone())
            .unwrap_or_default()
    } else {
        branch
    };

    let mut result = FlakeScanResult {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        repo,
        branch: branch_value,
        workflow_name: workflow_filter,
        summary,
        metrics,
        signals,
        trend: None,
    };
    result.trend = db::latest_comparable_scan(&result.repo, &result.branch, &result.workflow_name)
        .and_then(|previous| compute_trend(&result, Some(&previous)));

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testish_detection_catches_common_test_labels() {
        assert!(is_testish("Run integration tests"));
        assert!(is_testish("cargo test"));
        assert!(!is_testish("Set up job"));
    }

    #[test]
    fn signal_requires_fail_and_pass_overlap() {
        let mut bucket = SignalBucket {
            kind: "step".into(),
            workflow_name: "CI".into(),
            job_name: "linux".into(),
            step_name: "Run tests".into(),
            failure_count: 2,
            success_count: 2,
            rerun_hits: 1,
            ..SignalBucket::default()
        };
        bucket
            .fail_envs
            .insert("ubuntu-latest".into(), 2);
        bucket
            .success_envs
            .insert("ubuntu-22.04".into(), 2);

        let signal = build_signal(bucket).expect("signal");
        assert_eq!(signal.status, "quarantine");
        assert!(signal.score > 0);
    }
}
