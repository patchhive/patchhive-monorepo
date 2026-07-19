pub mod analysis;
mod discovery;
pub mod routes;
mod scan_hygiene;
pub mod scanning;
pub mod schedules;

// Re-export all public route handlers for main.rs.
pub use routes::{
    add_repo_list, auth_status, capabilities, delete_scan_preset, delete_scan_schedule, gen_key,
    gen_service_token, health, history, history_detail, login, overview, remove_repo_list,
    repo_lists, rotate_service_token, run_scan_schedule_now, runs, save_scan_preset,
    save_scan_schedule, scan_local_repo, scan_presets, scan_schedules, startup_checks_route,
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
const D = "service unavailable while syncing billing customers";
"#;

        let opportunities = analyze_file("src/client.ts", "typescript", source);
        assert!(opportunities
            .iter()
            .any(|item| item.kind == "repeated_literal" && item.safety == "medium"));
    }

    #[test]
    fn three_general_literals_do_not_enter_the_review_queue() {
        let source = r#"
const A = "service unavailable while syncing billing customers";
const B = "service unavailable while syncing billing customers";
const C = "service unavailable while syncing billing customers";
"#;

        assert!(!analyze_file("src/client.ts", "typescript", source)
            .iter()
            .any(|item| item.kind == "repeated_literal"));
    }

    #[test]
    fn five_contract_literals_need_one_consistent_usage_role_for_high_confidence() {
        let source = r#"
const A = { headers: { "Content-Type": "application/vnd.github+json" } };
const B = { headers: { "Content-Type": "application/vnd.github+json" } };
const C = { headers: { "Content-Type": "application/vnd.github+json" } };
const D = { headers: { "Content-Type": "application/vnd.github+json" } };
const E = { headers: { "Content-Type": "application/vnd.github+json" } };
"#;

        let opportunities = analyze_file("src/client.ts", "typescript", source);
        assert!(opportunities
            .iter()
            .any(|item| item.kind == "repeated_literal" && item.safety == "high"));
    }

    #[test]
    fn contract_shape_and_count_do_not_promote_declarative_registry_keys() {
        let source = r#"
const A = { key: "BOT_GITHUB_TOKEN", label: "SignalHive" };
const B = { key: "BOT_GITHUB_TOKEN", label: "TrustGate" };
const C = { key: "BOT_GITHUB_TOKEN", label: "RepoMemory" };
const D = { key: "BOT_GITHUB_TOKEN", label: "ReviewBee" };
const E = { key: "BOT_GITHUB_TOKEN", label: "MergeKeeper" };
"#;

        let opportunity = analyze_file("src/registry.ts", "typescript", source)
            .into_iter()
            .find(|item| item.kind == "repeated_literal")
            .expect("declarative contract usage should remain reviewable");
        assert_eq!(opportunity.safety, "medium");
        assert!(opportunity
            .evidence
            .iter()
            .any(|item| item.contains("does not establish one shared ownership boundary")));
    }

    #[test]
    fn occurrence_count_alone_does_not_make_literal_high_confidence() {
        let source = r#"
const A = "service unavailable while syncing billing customers";
const B = "service unavailable while syncing billing customers";
const C = "service unavailable while syncing billing customers";
const D = "service unavailable while syncing billing customers";
const E = "service unavailable while syncing billing customers";
"#;

        let opportunity = analyze_file("src/client.ts", "typescript", source)
            .into_iter()
            .find(|item| item.kind == "repeated_literal")
            .expect("repeated usage should remain reviewable");
        assert_eq!(opportunity.safety, "medium");
        assert!(opportunity.summary.contains("semantic ownership"));
    }

    #[test]
    fn repeated_literal_scan_does_not_cross_between_neighboring_strings() {
        let source = r#"
if text.contains("dependabot alerts are disabled")
    || text.contains("code scanning is not enabled")
    || text.contains("advanced security must be enabled")
{
    return FeatureDisabled;
}
if text.contains("resource not accessible by personal access token")
    || text.contains("requires authentication")
    || text.contains("missing token scope")
{
    return MissingScope;
}
"#;

        let opportunities = analyze_file("src/errors.rs", "rust", source);
        assert!(!opportunities
            .iter()
            .any(|item| item.kind == "repeated_literal"));
    }

    #[test]
    fn repeated_literal_scan_does_not_cross_same_line_neighboring_strings() {
        let source = r#"
if haystack.contains("cargo") || haystack.contains("crates.io") {}
if haystack.contains("pip") || haystack.contains("python") {}
if haystack.contains("npm") || haystack.contains("pnpm") {}
if haystack.contains("github action") || haystack.contains("actions/") {}
"#;

        let opportunities = analyze_file("src/utils.rs", "rust", source);
        assert!(!opportunities
            .iter()
            .any(|item| item.kind == "repeated_literal"));
    }

    #[test]
    fn repeated_literal_scan_ignores_rust_char_literals_before_neighboring_strings() {
        let source = r#"
fn clean_version_token(raw: &str) -> &str {
    raw.trim_matches(|character: char| {
        matches!(character, '`' | '"' | '\'' | ',' | '.' | ';' | ':' | '(' | ')' | '[' | ']')
    })
}

fn ecosystem(haystack: &str) -> &str {
    if haystack.contains("cargo") || haystack.contains("crates.io") {
        "rust"
    } else if haystack.contains("pip") || haystack.contains("python") {
        "python"
    } else if haystack.contains("npm") || haystack.contains("pnpm") {
        "javascript"
    } else {
        "unknown"
    }
}
"#;

        let opportunities = analyze_file("src/utils.rs", "rust", source);
        assert!(!opportunities.iter().any(|item| {
            item.kind == "repeated_literal" && item.summary.contains("haystack.contains")
        }));
    }

    #[test]
    fn repeated_literal_scan_ignores_css_tokens_and_class_lists() {
        let source = r#"
const A = { color: "var(--accent)", className: "surface-inset rounded-xl p-4" };
const B = { color: "var(--accent)", className: "surface-inset rounded-xl p-4" };
const C = { color: "var(--accent)", className: "surface-inset rounded-xl p-4" };
const D = { grid: "repeat(auto-fit, minmax(220px, 1fr))", transform: "translate(-50%, -50%)" };
const E = { grid: "repeat(auto-fit, minmax(220px, 1fr))", transform: "translate(-50%, -50%)" };
const F = { grid: "repeat(auto-fit, minmax(220px, 1fr))", transform: "translate(-50%, -50%)" };
"#;

        let opportunities = analyze_file("src/panel.jsx", "javascript", source);
        assert!(!opportunities
            .iter()
            .any(|item| item.kind == "repeated_literal"));
    }

    #[test]
    fn repeated_literal_scan_uses_usage_context_for_css_classes() {
        let source = r#"
const A = <button className="selected-stat">One</button>;
const B = <button className="selected-stat">Two</button>;
const C = <button className="selected-stat">Three</button>;
const D = <button className="selected-stat">Four</button>;
"#;

        assert!(!analyze_file("src/panel.jsx", "javascript", source)
            .iter()
            .any(|item| item.kind == "repeated_literal"));
    }

    #[test]
    fn repeated_literal_scan_ignores_tailwind_attribute_selectors() {
        let source = r#"
const A = "group-data-[collapsible=icon]:hidden";
const B = "group-data-[collapsible=icon]:hidden";
const C = "group-data-[collapsible=icon]:hidden";
const D = "group-data-[collapsible=icon]:hidden";
const E = "group-data-[collapsible=icon]:hidden";
"#;

        let opportunities = analyze_file("src/sidebar.tsx", "typescript", source);
        assert!(!opportunities
            .iter()
            .any(|item| item.kind == "repeated_literal"));
    }

    #[test]
    fn repeated_literal_scan_keeps_hyphenated_prose() {
        let source = r#"
const A = "service-token auth is not configured for this product";
const B = "service-token auth is not configured for this product";
const C = "service-token auth is not configured for this product";
const D = "service-token auth is not configured for this product";
"#;

        let opportunity = analyze_file("src/auth.rs", "rust", source)
            .into_iter()
            .find(|item| item.kind == "repeated_literal")
            .expect("hyphenated prose is not a CSS class list");
        assert!(opportunity.summary.contains("service-token auth"));
    }

    #[test]
    fn generated_source_does_not_create_refactor_leads() {
        let mut source = String::from("// This file was automatically generated. Do not edit.\n");
        source.push_str("export function generatedRoute() {\n");
        for _ in 0..90 {
            source.push_str("  console.log(\"generated route identifier\");\n");
        }
        source.push_str("}\n");

        assert!(analyze_file("src/routeTree.gen.ts", "typescript", &source).is_empty());
    }

    #[test]
    fn embedded_stylesheet_gets_stylesheet_guidance() {
        let mut source = String::from("function CommandCenterStyles() {\n  return <style>{`\n");
        for index in 0..210 {
            source.push_str(&format!("  .selector-{index} {{ color: red; }}\n"));
        }
        source.push_str("  `}</style>;\n}\n");

        let opportunity = analyze_file("src/SetupPanel.jsx", "javascript", &source)
            .into_iter()
            .find(|item| item.kind == "long_function")
            .expect("embedded stylesheet should remain visible");
        assert_eq!(
            opportunity.title,
            "Review embedded stylesheet boundary in `CommandCenterStyles`"
        );
        assert!(opportunity.suggestion.contains("CSS module"));
        assert!(opportunity
            .summary
            .contains("not a complex-function finding"));
    }

    #[test]
    fn repeated_literal_scan_handles_escaped_quotes() {
        let source = r#"
const A: &str = "service said \"try again later\"";
const B: &str = "service said \"try again later\"";
const C: &str = "service said \"try again later\"";
const D: &str = "service said \"try again later\"";
"#;

        let opportunities = analyze_file("src/messages.rs", "rust", source);
        let repeated = opportunities
            .iter()
            .find(|item| item.kind == "repeated_literal")
            .expect("real repeated literal should remain visible");
        assert!(repeated.summary.contains("service said"));
        assert!(!repeated.summary.contains("const B"));
    }

    #[test]
    fn rust_attribute_literals_are_not_extract_constant_leads() {
        let source = r#"
#[derive(serde::Deserialize)]
struct Example {
    #[serde(default, deserialize_with = "default_on_null")]
    first: String,
    #[serde(
        default,
        deserialize_with = "default_on_null"
    )]
    second: String,
    #[serde(default, deserialize_with = "default_on_null")]
    third: String,
}
"#;

        let opportunities = analyze_file("src/models.rs", "rust", source);
        assert!(!opportunities
            .iter()
            .any(|item| item.kind == "repeated_literal"));
    }

    #[test]
    fn inline_rust_test_literals_do_not_hide_runtime_leads_after_the_module() {
        let source = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn one() {
        assert_eq!("patchhive/example", "patchhive/example");
    }

    #[test]
    fn two() {
        let payload = serde_json::json!({ "repo": "patchhive/example" });
        assert_eq!(payload["repo"], "patchhive/example");
    }
}

const A: &str = "runtime service boundary message";
const B: &str = "runtime service boundary message";
const C: &str = "runtime service boundary message";
const D: &str = "runtime service boundary message";
"#;

        let opportunities = analyze_file("src/contract.rs", "rust", source);
        let repeated = opportunities
            .iter()
            .find(|item| item.kind == "repeated_literal")
            .expect("runtime literal after the test module should remain visible");
        assert!(repeated
            .summary
            .contains("runtime service boundary message"));
        assert!(!repeated.summary.contains("patchhive/example"));
    }

    #[test]
    fn repeated_validation_gets_context_aware_guidance() {
        let source = r#"
fn validate_one(repo: &str) -> anyhow::Result<()> {
    if !valid_repo(repo) {
        return Err(anyhow::anyhow!("Repository must be in owner/name format"));
    }
    Ok(())
}
fn validate_two(repo: &str) -> anyhow::Result<()> {
    if !valid_repo(repo) {
        return Err(anyhow::anyhow!("Repository must be in owner/name format"));
    }
    Ok(())
}
fn validate_three(repo: &str) -> anyhow::Result<()> {
    if !valid_repo(repo) {
        return Err(anyhow::anyhow!("Repository must be in owner/name format"));
    }
    Ok(())
}
"#;

        let opportunities = analyze_file("src/client.rs", "rust", source);
        let validation = opportunities
            .iter()
            .find(|item| item.kind == "repeated_validation")
            .expect("repeated validation should receive specific guidance");
        assert_eq!(validation.title, "Review repeated validation boundary");
        assert!(validation.suggestion.contains("Compare the guards"));
        assert!(validation.summary.contains("one contract"));
    }

    #[test]
    fn machine_readable_error_values_are_not_called_validation_messages() {
        let source = r#"
fn closed_client() -> Result<(), Error> {
    raise Error("connection_closed");
}
fn closed_socket() -> Result<(), Error> {
    raise Error("connection_closed");
}
fn closed_transport() -> Result<(), Error> {
    raise Error("connection_closed");
}
fn closed_session() -> Result<(), Error> {
    raise Error("connection_closed");
}
"#;

        let contract = analyze_file("src/client.py", "python", source)
            .into_iter()
            .find(|item| item.kind == "repeated_validation")
            .expect("repeated machine-readable errors should remain reviewable");
        assert_eq!(contract.title, "Review repeated error contract");
        assert!(contract.summary.contains("machine-readable error value"));
        assert!(!contract.summary.contains("validation message"));
        assert!(contract.suggestion.contains("exact machine-readable value"));
    }

    #[test]
    fn inline_rust_test_modules_do_not_inflate_file_or_function_measurements() {
        let mut source = String::from("pub fn runtime() {}\n\n#[cfg(test)]\nmod tests {\n");
        source.push_str("    #[test]\n    fn oversized_test_helper() {\n");
        for _ in 0..360 {
            source.push_str("        assert!(true);\n");
        }
        source.push_str("    }\n}\n\npub fn after_tests() {}\n");

        let opportunities = analyze_file("src/pipeline.rs", "rust", &source);
        assert!(!opportunities.iter().any(|item| item.kind == "large_file"));
        assert!(!opportunities
            .iter()
            .any(|item| item.kind == "long_function"));
    }

    #[test]
    fn runtime_code_after_inline_tests_still_counts_toward_file_size() {
        let mut source = String::from("#[cfg(test)]\nmod tests {\n");
        for _ in 0..80 {
            source.push_str("    // test support\n");
        }
        source.push_str("}\n");
        for _ in 0..330 {
            source.push_str("pub const RUNTIME_VALUE: bool = true;\n");
        }

        let opportunity = analyze_file("src/client.rs", "rust", &source)
            .into_iter()
            .find(|item| item.kind == "large_file")
            .expect("runtime lines after the test module should still count");
        assert!(opportunity
            .evidence
            .iter()
            .any(|item| item.contains("inline test-module lines excluded")));
        assert!(opportunity.summary.contains("330 measured non-test lines"));
    }

    #[test]
    fn declarative_jsx_is_not_described_as_branching_complexity() {
        let mut source = String::from("function ProductPanel() {\n  return (\n    <section>\n");
        for index in 0..100 {
            source.push_str(&format!(
                "      <div className=\"row\">Item {index}</div>\n"
            ));
        }
        source.push_str("    </section>\n  );\n}\n");

        let opportunity = analyze_file("src/ProductPanel.jsx", "javascript", &source)
            .into_iter()
            .find(|item| item.kind == "long_function")
            .expect("large declarative component should remain visible");
        assert!(opportunity.title.contains("declarative component"));
        assert!(opportunity.summary.contains("declarative JSX"));
        assert!(opportunity.score < 70);
    }

    #[test]
    fn schema_blocks_receive_schema_specific_guidance() {
        let mut source = String::from(
            "fn init_db(connection: &Connection) {\n    connection.execute_batch(r#\"\n",
        );
        for index in 0..100 {
            source.push_str(&format!(
                "CREATE TABLE example_{index} (id INTEGER PRIMARY KEY);\n"
            ));
        }
        source.push_str("\"#).unwrap();\n}\n");

        let opportunity = analyze_file("src/db.rs", "rust", &source)
            .into_iter()
            .find(|item| item.kind == "long_function")
            .expect("large schema setup should remain visible");
        assert!(opportunity.title.contains("schema setup boundary"));
        assert!(opportunity.summary.contains("SQL or schema declarations"));
    }

    #[test]
    fn lookup_tables_receive_table_specific_guidance() {
        let mut source = String::from(
            "fn credential_requirements(product: &str) -> &'static [&'static str] {\n    match product {\n",
        );
        for index in 0..100 {
            source.push_str(&format!("        \"product-{index}\" => &[\"token\"],\n"));
        }
        source.push_str("        _ => &[],\n    }\n}\n");

        let opportunity = analyze_file("src/registry.rs", "rust", &source)
            .into_iter()
            .find(|item| item.kind == "long_function")
            .expect("large lookup table should remain visible");
        assert!(opportunity.title.contains("declarative table"));
        assert!(opportunity.summary.contains("table or match-heavy shape"));
        assert!(opportunity.score < 75);
    }

    #[test]
    fn struct_literal_tables_are_not_scored_as_complex_control_flow() {
        let mut source =
            String::from("fn credential_requirements() -> Vec<Requirement> {\n    vec![\n");
        for index in 0..65 {
            source.push_str(&format!(
                "        Requirement {{\n            key: \"KEY_{index}\",\n            description: \"Used for a product boundary\",\n        }},\n"
            ));
        }
        source.push_str("    ]\n}\n");

        let opportunity = analyze_file("src/launcher.rs", "rust", &source)
            .into_iter()
            .find(|item| item.kind == "long_function")
            .expect("large struct table should remain visible");
        assert!(opportunity.title.contains("declarative table"));
        assert!(opportunity.score < 75);
    }

    #[test]
    fn long_function_scores_preserve_size_order_without_immediate_saturation() {
        fn rust_function(lines: usize) -> String {
            let mut source = String::from("fn candidate() {\n");
            for _ in 0..lines {
                source.push_str("    do_work();\n");
            }
            source.push_str("}\n");
            source
        }

        let short = analyze_file("src/short.rs", "rust", &rust_function(90))
            .into_iter()
            .find(|item| item.kind == "long_function")
            .expect("short candidate");
        let medium = analyze_file("src/medium.rs", "rust", &rust_function(140))
            .into_iter()
            .find(|item| item.kind == "long_function")
            .expect("medium candidate");
        let long = analyze_file("src/long.rs", "rust", &rust_function(300))
            .into_iter()
            .find(|item| item.kind == "long_function")
            .expect("long candidate");

        assert!(short.score < medium.score);
        assert!(medium.score < long.score);
        assert!(long.score < 94);
    }

    #[test]
    fn short_straight_line_bodies_do_not_enter_the_review_queue() {
        let mut source = String::from("fn cohesive_setup() {\n");
        for index in 0..70 {
            source.push_str(&format!("    configure_step_{index}();\n"));
        }
        source.push_str("}\n");

        assert!(!analyze_file("src/setup.rs", "rust", &source)
            .iter()
            .any(|item| item.kind == "long_function"));
    }

    #[test]
    fn shorter_control_dense_bodies_remain_reviewable() {
        let mut source = String::from("fn branching_setup() {\n");
        for index in 0..20 {
            source.push_str(&format!(
                "    if condition_{index}() {{\n        configure_{index}();\n    }}\n"
            ));
        }
        source.push_str("}\n");

        let opportunity = analyze_file("src/setup.rs", "rust", &source)
            .into_iter()
            .find(|item| item.kind == "long_function")
            .expect("control-dense body should remain reviewable");
        assert!(opportunity.summary.contains("control-flow markers"));
    }

    #[test]
    fn test_and_fixture_leads_rank_below_runtime_equivalents() {
        let source = r#"
const A: &str = "shared production boundary message";
const B: &str = "shared production boundary message";
const C: &str = "shared production boundary message";
const D: &str = "shared production boundary message";
"#;

        let runtime = analyze_file("src/client.rs", "rust", source)
            .into_iter()
            .find(|item| item.kind == "repeated_literal")
            .expect("runtime lead should exist");
        let test = analyze_file("tests/client.rs", "rust", source)
            .into_iter()
            .find(|item| item.kind == "repeated_literal")
            .expect("test lead should remain visible");

        assert_eq!(runtime.score.saturating_sub(test.score), 10);
        assert!(test
            .evidence
            .iter()
            .any(|item| item.contains("ranked below runtime code")));
    }

    #[test]
    fn local_scan_ignores_dependency_build_and_cache_directories() {
        let base =
            std::env::temp_dir().join(format!("refactor-scout-ignore-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(base.join("src")).expect("source directory should exist");
        fs::write(base.join("src/lib.rs"), "pub fn visible() {}\n")
            .expect("source fixture should write");

        for directory in [
            "node_modules",
            "NODE_MODULES",
            "build",
            "dist",
            "target",
            ".next",
            ".pytest_cache",
            "__pycache__",
            ".gradle",
            ".svelte-kit",
            "storybook-static",
        ] {
            let ignored = base.join(directory);
            fs::create_dir_all(&ignored).expect("ignored directory should exist");
            fs::write(
                ignored.join("noise.rs"),
                r#"
const A: &str = "ignored dependency or build output";
const B: &str = "ignored dependency or build output";
const C: &str = "ignored dependency or build output";
"#,
            )
            .expect("ignored fixture should write");
        }

        let state = AppState {
            allowed_roots: vec![base.clone()],
            remote_fs_enabled: false,
            ..AppState::new()
        };
        let result = build_scan_result(&state, base.to_str().expect("utf8 scan root"), 100)
            .expect("scan should succeed");

        assert_eq!(result.metrics.files_scanned, 1);
        assert_eq!(result.metrics.opportunities, 0);
        fs::remove_dir_all(base).ok();
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
    fn scan_retains_every_detected_opportunity() {
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
const D: &str = "repeated service boundary literal";
"#,
            )
            .expect("fixture should write");
        }
        let state = AppState {
            allowed_roots: vec![base.clone()],
            remote_fs_enabled: false,
            ..AppState::new()
        };

        let result = build_scan_result(&state, base.to_str().expect("utf8 scan root"), 100)
            .expect("scan should succeed");

        assert_eq!(result.metrics.opportunities, 65);
        assert_eq!(result.metrics.returned_opportunities, 65);
        assert!(!result.metrics.opportunities_truncated);
        assert_eq!(result.opportunities.len(), 65);

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
fn one() { let _ = ("Content-Type", "application/vnd.github+json"); }
fn two() { let _ = ("Content-Type", "application/vnd.github+json"); }
fn three() { let _ = ("Content-Type", "application/vnd.github+json"); }
fn four() { let _ = ("Content-Type", "application/vnd.github+json"); }
fn five() { let _ = ("Content-Type", "application/vnd.github+json"); }
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
            ..AppState::new()
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
        assert!(summary.contains("did not find structural review candidates"));
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
            ..AppState::new()
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
