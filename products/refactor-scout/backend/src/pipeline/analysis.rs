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
        returned_opportunities: opportunities.len() as u32,
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
            "repeated_literal" | "repeated_validation" => metrics.repeated_literal_count += 1,
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
            "RefactorScout did not find structural review candidates in `{repo_name}` within the current scan limits."
        );
    }

    let mut summary = format!(
        "RefactorScout found {} review candidate{} across {} scanned file{}. {} high-confidence candidate{}, {} candidate{} needing closer review.",
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
        summary.push_str(" Top review priority: ");
        summary.push_str(top.summary.trim_end_matches(['.', '!', '?']));
        summary.push('.');
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

pub(crate) fn scan_request_allowed(
    peer_addr: Option<std::net::SocketAddr>,
    remote_fs_enabled: bool,
) -> bool {
    if remote_fs_enabled {
        return true;
    }
    peer_addr.is_some_and(|addr| addr.ip().is_loopback())
}
