use std::collections::{BTreeMap, HashMap};

use chrono::Utc;
use uuid::Uuid;

use crate::{
    db, github,
    github::{GitHubWorkflowJob, GitHubWorkflowRun},
    models::{compute_trend, FlakeMetrics, FlakeScanResult, FlakeSignal},
    state::AppState,
};

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

fn normalized_runner_label(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }

    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() > 2
        && parts[0].eq_ignore_ascii_case("GitHub")
        && parts[1].eq_ignore_ascii_case("Actions")
        && parts[2..]
            .iter()
            .all(|part| part.chars().all(|character| character.is_ascii_digit()))
    {
        return Some("GitHub Actions".into());
    }

    Some(value.into())
}

fn runner_label(job: &GitHubWorkflowJob) -> String {
    let labels = job
        .labels
        .iter()
        .filter_map(|label| normalized_runner_label(label))
        .filter(|label| !label.eq_ignore_ascii_case("GitHub Actions"))
        .collect::<Vec<_>>();
    let runner = normalized_runner_label(&job.runner_name);

    if matches!(runner.as_deref(), Some(value) if value.eq_ignore_ascii_case("GitHub Actions"))
        && !labels.is_empty()
    {
        return labels.join(", ");
    }

    runner
        .or_else(|| (!labels.is_empty()).then(|| labels.join(", ")))
        .unwrap_or_else(|| "unknown-runner".into())
}

fn push_evidence(items: &mut Vec<String>, value: String) {
    if value.trim().is_empty() || items.iter().any(|item| item == &value) {
        return;
    }
    if items.len() < 6 {
        items.push(value);
    }
}

struct BucketObservation<'a> {
    kind: &'a str,
    workflow_name: &'a str,
    job_name: &'a str,
    step_name: &'a str,
    conclusion: &'a str,
    run: &'a GitHubWorkflowRun,
    runner: &'a str,
    evidence_url: &'a str,
}

fn record_bucket(buckets: &mut HashMap<String, SignalBucket>, item: BucketObservation<'_>) {
    let key = format!(
        "{}:{}:{}:{}",
        item.kind, item.workflow_name, item.job_name, item.step_name
    );
    let bucket = buckets.entry(key).or_default();
    if bucket.kind.is_empty() {
        bucket.kind = item.kind.into();
        bucket.workflow_name = item.workflow_name.into();
        bucket.job_name = item.job_name.into();
        bucket.step_name = item.step_name.into();
    }

    if is_failure(item.conclusion) {
        bucket.failure_count += 1;
        *bucket.fail_envs.entry(item.runner.into()).or_default() += 1;
    } else if is_success(item.conclusion) {
        bucket.success_count += 1;
        *bucket.success_envs.entry(item.runner.into()).or_default() += 1;
    } else {
        return;
    }

    if item.run.run_attempt > 1 {
        bucket.rerun_hits += 1;
    }

    let line = format!(
        "run #{} attempt {} → {} on {}{}{}",
        item.run.run_number,
        item.run.run_attempt.max(1),
        if item.conclusion.trim().is_empty() {
            "unknown"
        } else {
            item.conclusion
        },
        item.runner,
        if item.run.html_url.trim().is_empty() {
            ""
        } else {
            " · "
        },
        item.evidence_url,
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
    if environment_hints(bucket)
        .iter()
        .any(|hint| hint.contains("clustering"))
    {
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

pub(super) async fn build_scan_result(
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
                    BucketObservation {
                        kind: "step",
                        workflow_name: &run.name,
                        job_name: &job.name,
                        step_name: &step.name,
                        conclusion: &step.conclusion,
                        run,
                        runner: &runner,
                        evidence_url: &job.html_url,
                    },
                );
            }

            if !recorded_step
                && job_testish
                && (is_success(&job.conclusion) || is_failure(&job.conclusion))
            {
                record_bucket(
                    &mut buckets,
                    BucketObservation {
                        kind: "job",
                        workflow_name: &run.name,
                        job_name: &job.name,
                        step_name: "",
                        conclusion: &job.conclusion,
                        run,
                        runner: &runner,
                        evidence_url: &job.html_url,
                    },
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
    let summary = if metrics.workflow_runs == 0 {
        format!(
            "FlakeSting did not find matching completed workflow runs for `{repo}`. No CI instability evidence was available to rank for this scan."
        )
    } else if signals.is_empty() {
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
        bucket.fail_envs.insert("ubuntu-latest".into(), 2);
        bucket.success_envs.insert("ubuntu-22.04".into(), 2);

        let signal = build_signal(bucket).expect("signal");
        assert_eq!(signal.status, "quarantine");
        assert!(signal.score > 0);
    }

    #[test]
    fn runner_label_prefers_action_runner_labels_over_opaque_ids() {
        let job = GitHubWorkflowJob {
            runner_name: "GitHub Actions 1000002204".into(),
            labels: vec!["ubuntu-latest".into()],
            ..GitHubWorkflowJob::default()
        };

        assert_eq!(runner_label(&job), "ubuntu-latest");
    }
}
