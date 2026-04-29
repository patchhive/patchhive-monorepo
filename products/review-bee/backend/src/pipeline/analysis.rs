use crate::models::ChecklistEvidence;

pub(crate) fn checklist_title(category: &str, path_hint: &str) -> String {
    let area = if path_hint == "general" {
        String::new()
    } else {
        format!(" in {path_hint}")
    };

    match category {
        "tests" => format!("Add or adjust tests{area}"),
        "validation" => format!("Strengthen validation and guards{area}"),
        "naming" => format!("Align naming or structure{area}"),
        "docs" => format!("Update docs or supporting context{area}"),
        "cleanup" => format!("Simplify or clean up the implementation{area}"),
        "errors" => format!("Tighten error handling{area}"),
        "api" => format!("Clarify API or behavior contracts{area}"),
        "performance" => format!("Address performance or query concerns{area}"),
        "style" => format!("Smooth out style or consistency concerns{area}"),
        _ => format!("Address remaining review feedback{area}"),
    }
}

pub(crate) fn prompt_hint(category: &str, path_hints: &[String]) -> String {
    let area = if path_hints.is_empty() {
        "the touched code".into()
    } else {
        path_hints.join(", ")
    };

    match category {
        "tests" => format!("Resolve review feedback by adding or tightening tests around {area}."),
        "validation" => format!("Resolve review feedback by restoring validation and guard-rail coverage around {area}."),
        "naming" => format!("Resolve review feedback by matching local naming and structure conventions in {area}."),
        "docs" => format!("Resolve review feedback by syncing docs, comments, or explanatory context in {area}."),
        "cleanup" => format!("Resolve review feedback by simplifying the implementation in {area} and removing one-off logic."),
        "errors" => format!("Resolve review feedback by improving error handling and context in {area}."),
        "api" => format!("Resolve review feedback by clarifying behavior contracts and edge cases in {area}."),
        "performance" => format!("Resolve review feedback by checking the performance impact of changes in {area}."),
        "style" => format!("Resolve review feedback by matching the repo's style expectations in {area}."),
        _ => format!("Resolve the remaining review feedback in {area} before merge."),
    }
}

pub(crate) fn actionable_text(text: &str) -> bool {
    let compact = collapse_whitespace(text);
    if compact.len() < 10 {
        return false;
    }
    let lower = compact.to_ascii_lowercase();
    let request_terms = [
        "please",
        "need",
        "needs",
        "should",
        "must",
        "can you",
        "could you",
        "would you",
        "consider",
        "instead",
        "avoid",
        "prefer",
        "use ",
        "remove",
        "rename",
        "handle",
        "update",
        "add ",
        "include",
        "cover",
        "fix",
        "missing",
        "nit:",
        "nit ",
    ];
    let praise_terms = [
        "lgtm",
        "looks good",
        "nice work",
        "great work",
        "great catch",
        "thanks",
        "thank you",
        "awesome",
    ];

    if contains_any(&lower, &request_terms) {
        return true;
    }

    if lower.contains('?')
        && contains_any(
            &lower,
            &[
                "can",
                "could",
                "would",
                "should",
                "why",
                "what about",
                "do we",
            ],
        )
    {
        return true;
    }

    !contains_any(&lower, &praise_terms) && lower.split_whitespace().count() >= 6
}

pub(crate) fn classify_category(text: &str) -> (&'static str, &'static str) {
    let lower = text.to_ascii_lowercase();

    if contains_any(&lower, &["test", "coverage", "assert", "spec", "fixture"]) {
        ("tests", "Tests")
    } else if contains_any(
        &lower,
        &[
            "validate",
            "validation",
            "guard",
            "sanitize",
            "edge case",
            "null",
            "nil",
        ],
    ) {
        ("validation", "Validation")
    } else if contains_any(
        &lower,
        &[
            "rename",
            "naming",
            "consistent",
            "convention",
            "structure",
            "pattern",
        ],
    ) {
        ("naming", "Naming")
    } else if contains_any(
        &lower,
        &["readme", "docs", "document", "comment", "explain"],
    ) {
        ("docs", "Docs")
    } else if contains_any(
        &lower,
        &[
            "refactor", "simplify", "clean up", "cleanup", "helper", "reuse",
        ],
    ) {
        ("cleanup", "Cleanup")
    } else if contains_any(
        &lower,
        &["error", "logging", "log ", "fallback", "retry", "panic"],
    ) {
        ("errors", "Error handling")
    } else if contains_any(
        &lower,
        &[
            "api",
            "contract",
            "behavior",
            "return",
            "response",
            "interface",
        ],
    ) {
        ("api", "API behavior")
    } else if contains_any(
        &lower,
        &["slow", "performance", "query", "n+1", "efficient"],
    ) {
        ("performance", "Performance")
    } else if contains_any(&lower, &["style", "format", "lint", "indent", "spacing"]) {
        ("style", "Style")
    } else {
        ("general", "General")
    }
}

pub(crate) fn category_label(category: &str) -> &'static str {
    match category {
        "tests" => "tests",
        "validation" => "validation",
        "naming" => "naming / structure",
        "docs" => "docs",
        "cleanup" => "cleanup",
        "errors" => "error handling",
        "api" => "API behavior",
        "performance" => "performance",
        "style" => "style",
        _ => "general follow-up",
    }
}

pub(crate) fn path_bucket(path: &str) -> String {
    let trimmed = path.trim().trim_start_matches("./");
    if trimmed.is_empty() {
        return "general".into();
    }

    let parts = trimmed.split('/').collect::<Vec<_>>();
    if parts.len() >= 2
        && matches!(
            parts[0],
            "src" | "app" | "lib" | "server" | "backend" | "frontend" | "tests" | "packages"
        )
    {
        return format!("{}/{}", parts[0], parts[1]);
    }

    parts.first().copied().unwrap_or(trimmed).to_string()
}

pub(crate) fn status_rank(status: &str) -> i32 {
    match status {
        "open" => 3,
        "mixed" => 2,
        "resolved" => 1,
        _ => 0,
    }
}

pub(crate) fn plural_suffix(count: u32) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

pub(crate) fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub(crate) fn truncate(value: &str, limit: usize) -> String {
    let compact = collapse_whitespace(value);
    if compact.chars().count() <= limit {
        compact
    } else {
        compact
            .chars()
            .take(limit.saturating_sub(1))
            .collect::<String>()
            + "\u{2026}"
    }
}

pub(crate) fn push_evidence(items: &mut Vec<ChecklistEvidence>, evidence: ChecklistEvidence) {
    if items
        .iter()
        .any(|existing| existing.url == evidence.url && existing.excerpt == evidence.excerpt)
    {
        return;
    }
    if items.len() < 5 {
        items.push(evidence);
    }
}

pub(crate) fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}
