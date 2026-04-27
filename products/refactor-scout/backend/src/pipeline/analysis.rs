use crate::models::{RefactorOpportunity, ScanMetrics};

pub(crate) fn build_metrics(
    files_scanned: u32,
    files_skipped: u32,
    opportunities: &[RefactorOpportunity],
) -> ScanMetrics {
    let mut metrics = ScanMetrics {
        files_scanned,
        files_skipped,
        opportunities: opportunities.len() as u32,
        ..ScanMetrics::default()
    };

    for opportunity in opportunities {
        match opportunity.safety.as_str() {
            "high" => metrics.high_safety += 1,
            _ => metrics.medium_safety += 1,
        }

        match opportunity.kind.as_str() {
            "large_file" => metrics.large_file_count += 1,
            "long_function" => metrics.long_function_count += 1,
            "repeated_literal" => metrics.repeated_literal_count += 1,
            _ => {}
        }
    }

    metrics
}

pub(crate) fn build_summary(
    repo_name: &str,
    metrics: &ScanMetrics,
    top: Option<&RefactorOpportunity>,
) -> String {
    if metrics.opportunities == 0 {
        return format!(
            "RefactorScout did not find clear low-risk refactor candidates in `{repo_name}` within the current scan limits."
        );
    }

    let mut summary = format!(
        "RefactorScout found {} candidate{} across {} scanned file{}. {} high-safety lead{}, {} medium-safety lead{}.",
        metrics.opportunities,
        plural_suffix(metrics.opportunities),
        metrics.files_scanned,
        plural_suffix(metrics.files_scanned),
        metrics.high_safety,
        plural_suffix(metrics.high_safety),
        metrics.medium_safety,
        plural_suffix(metrics.medium_safety),
    );

    if let Some(top) = top {
        summary.push_str(&format!(" Strongest lead: {}.", top.summary));
    }

    summary
}

pub(crate) fn safety_rank(safety: &str) -> u8 {
    match safety {
        "high" => 2,
        _ => 1,
    }
}

pub(crate) fn plural_suffix(value: u32) -> &'static str {
    if value == 1 {
        ""
    } else {
        "s"
    }
}

pub(crate) fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < super::scanning::MAX_WARNINGS {
        warnings.push(warning);
    }
}

pub(crate) fn scan_request_allowed(headers: &axum::http::HeaderMap, remote_fs_enabled: bool) -> bool {
    if remote_fs_enabled {
        return true;
    }

    let mut saw_browser_local_hint = false;

    for header in ["origin", "referer"] {
        if let Some(value) = headers.get(header).and_then(|value| value.to_str().ok()) {
            if !local_endpoint(value) {
                return false;
            }
            saw_browser_local_hint = true;
        }
    }

    if saw_browser_local_hint {
        return true;
    }

    if let Some(value) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        let client = value.split(',').next().unwrap_or("").trim();
        if !matches!(client, "" | "127.0.0.1" | "::1" | "[::1]") {
            return false;
        }
    }

    headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .map(local_endpoint)
        .unwrap_or(false)
}

fn local_endpoint(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let without_scheme = trimmed
        .split("://")
        .nth(1)
        .unwrap_or(trimmed)
        .split('/')
        .next()
        .unwrap_or(trimmed)
        .trim();

    let host = without_scheme
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(':')
        .next()
        .unwrap_or(without_scheme)
        .trim();

    matches!(host, "localhost" | "127.0.0.1" | "::1")
}
