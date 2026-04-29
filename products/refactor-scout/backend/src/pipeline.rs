mod analysis;
mod routes;
mod scanning;

// Re-export all public route handlers for main.rs.
pub use routes::{
    auth_status, capabilities, gen_key, gen_service_token, health, history, history_detail, login,
    overview, rotate_service_token, runs, scan_local_repo, startup_checks_route,
};

#[cfg(test)]
mod tests {
    use super::analysis::build_summary;
    use super::scanning::{analyze_file, resolve_scan_root};
    use crate::{models::ScanMetrics, state::AppState};
    use std::{fs, path::PathBuf};

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

        fs::remove_dir_all(PathBuf::from(base)).ok();
    }
}
