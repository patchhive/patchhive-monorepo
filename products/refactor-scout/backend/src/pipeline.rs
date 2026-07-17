pub mod analysis;
pub mod routes;
pub mod scanning;

// Re-export all public route handlers for main.rs.
pub use routes::{
    auth_status, capabilities, gen_key, gen_service_token, health, history, history_detail, login,
    overview, rotate_service_token, runs, scan_local_repo, startup_checks_route,
};

#[cfg(test)]
mod tests {
    use super::analysis::build_summary;
    use super::scanning::{
        analyze_file, build_scan_result, parse_github_repo_target, resolve_scan_root,
    };
    use crate::{models::ScanMetrics, state::AppState};
    use std::fs;

    #[test]
    fn analyze_file_surfaces_large_file_and_long_function() {
        let mut source = String::from("fn huge_function() {\n");
        for line in 0..85 {
            source.push_str(&format!("    println!(\"line {line}\");\n"));
        }
        source.push_str("}\n");
        for _ in 0..260 {
            source.push_str("// filler\n");
        }

        let opportunities = analyze_file("src/lib.rs", "rust", &source);
        assert!(opportunities.iter().any(|item| item.kind == "large_file"));
        assert!(opportunities
            .iter()
            .any(|item| item.kind == "long_function"));
    }

    #[test]
    fn analyze_file_surfaces_repeated_literal_candidate() {
        let source = r#"
const A = "service unavailable while syncing billing customers";
const B = "service unavailable while syncing billing customers";
const C = "service unavailable while syncing billing customers";
"#;

        let opportunities = analyze_file("src/client.ts", "typescript", source);
        assert!(opportunities
            .iter()
            .any(|item| item.kind == "repeated_literal" && item.safety == "high"));
    }

    #[test]
    fn rust_braces_in_strings_and_comments_do_not_expand_function_bounds() {
        let mut source = String::from(
            r####"
fn compact_test() {
    assert!(written.contains(&format!("{service_env_var}=\"{{\\\"id\\\"")));
    let raw = r###"raw braces { do not open a function }"###;
    let brace = '{';
    // An unmatched { in a comment is not structural.
}
fn lifetime_value<'a>(value: &'a str) -> &'a str { value }
"####,
        );
        for _ in 0..80 {
            source.push_str("// module filler after the function\n");
        }

        let opportunities = analyze_file("src/auth.rs", "rust", &source);
        assert!(!opportunities
            .iter()
            .any(|item| item.kind == "long_function"));
    }

    #[test]
    fn scan_metrics_keep_total_when_returned_queue_is_bounded() {
        let base =
            std::env::temp_dir().join(format!("refactor-scout-cap-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&base).expect("scan root should exist");
        for index in 0..65 {
            fs::write(
                base.join(format!("candidate_{index}.rs")),
                r#"
const A: &str = "repeated service boundary literal";
const B: &str = "repeated service boundary literal";
const C: &str = "repeated service boundary literal";
"#,
            )
            .expect("fixture should write");
        }
        let state = AppState {
            allowed_roots: vec![base.clone()],
            remote_fs_enabled: false,
        };

        let result = build_scan_result(&state, base.to_str().expect("utf8 scan root"), 100)
            .expect("scan should succeed");

        assert_eq!(result.metrics.opportunities, 65);
        assert_eq!(result.metrics.returned_opportunities, 60);
        assert!(result.metrics.opportunities_truncated);
        assert_eq!(result.opportunities.len(), 60);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("found 65 candidates")));

        fs::remove_dir_all(base).ok();
    }

    #[test]
    fn scan_summary_uses_the_same_safety_first_priority_as_the_queue() {
        let base =
            std::env::temp_dir().join(format!("refactor-scout-priority-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&base).expect("scan root should exist");
        fs::write(
            base.join("bounded.rs"),
            r#"
const A: &str = "shared validation boundary message";
const B: &str = "shared validation boundary message";
const C: &str = "shared validation boundary message";
"#,
        )
        .expect("high-safety fixture should write");
        let mut long_function = String::from("fn oversized() {\n");
        for index in 0..140 {
            long_function.push_str(&format!("    println!(\"step {index}\");\n"));
        }
        long_function.push_str("}\n");
        fs::write(base.join("oversized.rs"), long_function)
            .expect("medium-safety fixture should write");
        let state = AppState {
            allowed_roots: vec![base.clone()],
            remote_fs_enabled: false,
        };

        let result = build_scan_result(&state, base.to_str().expect("utf8 scan root"), 100)
            .expect("scan should succeed");

        let top = result
            .opportunities
            .first()
            .expect("priority queue should not be empty");
        assert_eq!(top.kind, "repeated_literal");
        assert_eq!(top.safety, "high");
        assert!(result.summary.contains("Top review priority:"));
        assert!(
            result.summary.contains(&top.summary),
            "saved summary should describe the same top item: {}",
            result.summary
        );

        fs::remove_dir_all(base).ok();
    }

    #[test]
    fn build_summary_handles_empty_scan_cleanly() {
        let summary = build_summary("example", &ScanMetrics::default(), None);
        assert!(summary.contains("did not find clear low-risk refactor candidates"));
    }

    #[test]
    fn resolve_scan_root_rejects_paths_outside_allowed_roots() {
        let base =
            std::env::temp_dir().join(format!("refactor-scout-test-{}", uuid::Uuid::new_v4()));
        let allowed = base.join("allowed");
        let outside = base.join("outside");
        fs::create_dir_all(&allowed).expect("allowed dir should exist");
        fs::create_dir_all(&outside).expect("outside dir should exist");

        let state = AppState {
            allowed_roots: vec![allowed.clone()],
            remote_fs_enabled: false,
        };

        let err = resolve_scan_root(outside.to_str().expect("utf8 path"), &state)
            .expect_err("outside root should be rejected");
        assert!(err
            .to_string()
            .contains("outside the configured allowed roots"));

        fs::remove_dir_all(base).ok();
    }

    #[test]
    fn github_repo_targets_parse_common_inputs() {
        assert_eq!(
            parse_github_repo_target("patchhive/patchhive2")
                .map(|target| target.label())
                .as_deref(),
            Some("patchhive/patchhive2")
        );
        assert_eq!(
            parse_github_repo_target("https://github.com/patchhive/patchhive2.git")
                .map(|target| target.label())
                .as_deref(),
            Some("patchhive/patchhive2")
        );
        assert_eq!(
            parse_github_repo_target("git@github.com:patchhive/patchhive2.git")
                .map(|target| target.label())
                .as_deref(),
            Some("patchhive/patchhive2")
        );
        assert!(parse_github_repo_target("https://example.com/patchhive/patchhive2").is_none());
        assert!(parse_github_repo_target("patchhive/patchhive2/tree/main").is_none());
    }
}
