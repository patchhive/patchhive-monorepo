use std::collections::HashMap;

use serde_json::{json, Value};

use crate::models::{ProductOverride, ProductOverrideInput};

pub mod dispatch;
mod overview;
mod provision;
pub mod routes;
pub mod settings;
mod setup;
mod smoke;
pub mod types;

pub use types::{
    api_error, authorized_get, authorized_request, build_target_url, contract_check,
    contract_checks_for_failed_health, contract_checks_for_unavailable_product,
    contract_checks_with_health_error, contract_drift_count, fetch_product_auth_status,
    fetch_product_capabilities, fetch_product_runs, parse_response_body, persist_product_override,
    pick_url, remote_error_message, resolved_auth_mode, resolved_legacy_api_key_configured,
    resolved_machine_auth_configured, resolved_service_token_configured, DispatchActionInput,
    ProductAuthStatusBody, ProductHealthBody, ProductProbeSnapshot, ProductStoredAuth,
    StartupChecksBody,
};

pub(crate) use patchhive_product_core::auth::SERVICE_TOKEN_HEADER;

pub use routes::{
    auth_status, capabilities, dispatch_product_action, first_stack_status, gen_key,
    gen_service_token, health, login, overview, pair_first_stack, product_run_detail, product_runs,
    products, provision_service_token, recent_actions, restart_setup_product, rotate_service_token,
    run_detail, run_first_stack_smoke, runs, save_settings, settings, setup_product_logs,
    start_first_stack, start_setup_product, startup_checks_route, stop_first_stack,
    stop_setup_product,
};

fn hive_core_action_run_values(limit: u32) -> Vec<Value> {
    crate::db::recent_action_events(limit)
        .into_iter()
        .map(|event| {
            let summary = if event.error.is_empty() {
                format!(
                    "{} {} returned {}",
                    event.method,
                    event.path,
                    event
                        .remote_status
                        .map(|status| status.to_string())
                        .unwrap_or_else(|| "no remote status".into())
                )
            } else {
                event.error.clone()
            };
            json!({
                "id": event.id.clone(),
                "status": event.status.clone(),
                "title": format!("{} · {}", event.product_slug, event.action_label),
                "summary": summary,
                "created_at": event.created_at.clone(),
                "updated_at": event.created_at.clone(),
                "raw": event,
            })
        })
        .collect()
}

fn sanitize_product_overrides(
    products: Vec<ProductOverrideInput>,
    existing: &HashMap<String, ProductOverride>,
) -> Vec<ProductOverride> {
    settings::sanitize_product_overrides(products, existing)
}

fn dispatch_service_token_issue(
    product_title: &str,
    action: &patchhive_product_core::contract::ProductAction,
    auth_status: &ProductAuthStatusBody,
) -> Option<(&'static str, String)> {
    dispatch::dispatch_service_token_issue(product_title, action, auth_status)
}

fn parse_dispatch_input(raw: Value) -> DispatchActionInput {
    dispatch::parse_dispatch_input(raw)
}

fn fill_path_template(path: &str, path_params: &HashMap<String, String>) -> Result<String, String> {
    dispatch::fill_path_template(path, path_params)
}

fn build_run_detail_path(template: &str, id: &str) -> Result<String, String> {
    overview::build_run_detail_path(template, id)
}

fn summarize_products(
    products: &[crate::models::ProductRuntimeItem],
) -> crate::models::OverviewSummary {
    overview::summarize_products(products)
}

#[cfg(test)]
mod tests {
    use super::{
        authorized_request, build_run_detail_path, contract_check, contract_drift_count,
        dispatch_service_token_issue, fill_path_template, parse_dispatch_input, pick_url,
        sanitize_product_overrides, summarize_products, ProductAuthStatusBody, ProductStoredAuth,
        SERVICE_TOKEN_HEADER,
    };
    use crate::models::{
        now_rfc3339, ProductHealthSnapshot, ProductOverride, ProductOverrideInput,
        ProductRuntimeItem,
    };
    use patchhive_product_core::contract;
    use reqwest::Client;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn summarize_products_counts_each_runtime_status() {
        let products = vec![
            ProductRuntimeItem {
                slug: "signal-hive".into(),
                title: "SignalHive".into(),
                icon: "\u{1F4E1}".into(),
                lane: "Visibility".into(),
                role: "role".into(),
                repo: "patchhive/signalhive".into(),
                enabled: true,
                frontend_url: String::new(),
                api_url: String::new(),
                auth_mode: "none".into(),
                machine_auth_configured: false,
                service_token_configured: false,
                legacy_api_key_configured: false,
                notes: String::new(),
                status: "online".into(),
                health: ProductHealthSnapshot::default(),
                hivecore: None,
                actions: Vec::new(),
                links: Vec::new(),
                contract_checks: Vec::new(),
                contract_drift_count: 0,
                run_detail_template: String::new(),
                recent_runs: Vec::new(),
            },
            ProductRuntimeItem {
                slug: "repo-reaper".into(),
                title: "RepoReaper".into(),
                icon: "\u{2694}".into(),
                lane: "Action".into(),
                role: "role".into(),
                repo: "patchhive/reporeaper".into(),
                enabled: true,
                frontend_url: String::new(),
                api_url: String::new(),
                auth_mode: "none".into(),
                machine_auth_configured: false,
                service_token_configured: false,
                legacy_api_key_configured: false,
                notes: String::new(),
                status: "degraded".into(),
                health: ProductHealthSnapshot::default(),
                hivecore: None,
                actions: Vec::new(),
                links: Vec::new(),
                contract_checks: Vec::new(),
                contract_drift_count: 0,
                run_detail_template: String::new(),
                recent_runs: Vec::new(),
            },
            ProductRuntimeItem {
                slug: "review-bee".into(),
                title: "ReviewBee".into(),
                icon: "\u{1F41D}".into(),
                lane: "Review".into(),
                role: "role".into(),
                repo: "patchhive/reviewbee".into(),
                enabled: true,
                frontend_url: String::new(),
                api_url: String::new(),
                auth_mode: "none".into(),
                machine_auth_configured: false,
                service_token_configured: false,
                legacy_api_key_configured: false,
                notes: String::new(),
                status: "unconfigured".into(),
                health: ProductHealthSnapshot::default(),
                hivecore: None,
                actions: Vec::new(),
                links: Vec::new(),
                contract_checks: Vec::new(),
                contract_drift_count: 0,
                run_detail_template: String::new(),
                recent_runs: Vec::new(),
            },
            ProductRuntimeItem {
                slug: "merge-keeper".into(),
                title: "MergeKeeper".into(),
                icon: "\u{1F517}".into(),
                lane: "Merge".into(),
                role: "role".into(),
                repo: "patchhive/mergekeeper".into(),
                enabled: false,
                frontend_url: String::new(),
                api_url: String::new(),
                auth_mode: "none".into(),
                machine_auth_configured: false,
                service_token_configured: false,
                legacy_api_key_configured: false,
                notes: String::new(),
                status: "disabled".into(),
                health: ProductHealthSnapshot::default(),
                hivecore: None,
                actions: Vec::new(),
                links: Vec::new(),
                contract_checks: Vec::new(),
                contract_drift_count: 0,
                run_detail_template: String::new(),
                recent_runs: Vec::new(),
            },
        ];

        let summary = summarize_products(&products);
        assert_eq!(summary.total_products, 4);
        assert_eq!(summary.enabled_products, 3);
        assert_eq!(summary.online_products, 1);
        assert_eq!(summary.degraded_products, 1);
        assert_eq!(summary.unconfigured_products, 1);
        assert_eq!(summary.disabled_products, 1);
    }

    #[test]
    fn pick_url_prefers_non_empty_override() {
        assert_eq!(
            pick_url(Some(" https://example.com "), "http://localhost:8010"),
            "https://example.com"
        );
        assert_eq!(
            pick_url(Some(""), "http://localhost:8010"),
            "http://localhost:8010"
        );
        assert_eq!(
            pick_url(None, "http://localhost:8010"),
            "http://localhost:8010"
        );
    }

    #[test]
    fn parse_dispatch_input_accepts_wrapped_payloads() {
        let input = parse_dispatch_input(json!({
            "path_params": { "name": "daily" },
            "query": { "dry": true },
            "payload": { "repo": "patchhive/example" }
        }));

        assert_eq!(input.path_params["name"], "daily");
        assert_eq!(input.query["dry"], "true");
        assert_eq!(input.payload["repo"], "patchhive/example");
    }

    #[test]
    fn parse_dispatch_input_treats_plain_object_as_payload() {
        let input = parse_dispatch_input(json!({ "repo": "patchhive/example" }));
        assert_eq!(input.payload["repo"], "patchhive/example");
        assert!(input.path_params.is_empty());
    }

    #[test]
    fn fill_path_template_requires_all_path_params() {
        let mut params = HashMap::new();
        params.insert("name".into(), "daily".into());
        assert_eq!(
            fill_path_template("/schedules/{name}/run", &params).unwrap(),
            "/schedules/daily/run"
        );
        assert!(fill_path_template("/schedules/{missing}/run", &params).is_err());
    }

    #[test]
    fn build_run_detail_path_rejects_unsafe_ids() {
        assert_eq!(
            build_run_detail_path("/runs/{id}", "run_123").unwrap(),
            "/runs/run_123"
        );
        assert!(build_run_detail_path("/runs/{id}", "../secret").is_err());
        assert!(build_run_detail_path("/runs/{id}", "run?x=1").is_err());
    }

    #[test]
    fn contract_drift_ignores_operator_locked_states() {
        let checks = vec![
            contract_check("health", "Health", "/health", true, "ok", ""),
            contract_check("runs", "Runs", "/runs", false, "locked", "API key missing"),
            contract_check("detail", "Run detail", "/runs/{id}", false, "missing", ""),
        ];
        assert_eq!(contract_drift_count(&checks), 1);
    }

    #[test]
    fn sanitize_product_overrides_replaces_legacy_key_when_service_token_is_supplied() {
        let existing = HashMap::from([(
            "signal-hive".to_string(),
            ProductOverride {
                slug: "signal-hive".into(),
                frontend_url: String::new(),
                api_url: "http://localhost:8010".into(),
                service_token: String::new(),
                legacy_api_key: "signal-operator".into(),
                enabled: true,
                notes: String::new(),
                updated_at: now_rfc3339(),
            },
        )]);

        let sanitized = sanitize_product_overrides(
            vec![ProductOverrideInput {
                slug: "signal-hive".into(),
                frontend_url: String::new(),
                api_url: "http://localhost:8010".into(),
                service_token: Some("svc_signal".into()),
                legacy_api_key: None,
                enabled: true,
                notes: String::new(),
            }],
            &existing,
        );

        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0].service_token, "svc_signal");
        assert!(sanitized[0].legacy_api_key.is_empty());
    }

    #[test]
    fn authorized_request_prefers_service_token_header() {
        let request = authorized_request(
            Client::new().get("http://example.com"),
            &ProductStoredAuth {
                service_token: "svc_signal".into(),
                legacy_api_key: "signal-operator".into(),
            },
        )
        .build()
        .expect("request should build");

        assert_eq!(
            request
                .headers()
                .get(SERVICE_TOKEN_HEADER)
                .and_then(|value| value.to_str().ok()),
            Some("svc_signal")
        );
        assert!(request.headers().get("X-API-Key").is_none());
    }

    #[test]
    fn dispatch_service_token_issue_blocks_legacy_machine_tokens() {
        let action = contract::ProductAction {
            id: "scan".into(),
            label: "Run signal scan".into(),
            method: "POST".into(),
            path: "/scan".into(),
            description: String::new(),
            starts_run: true,
            destructive: false,
            required_scopes: vec!["actions:dispatch".into()],
        };
        let auth_status = ProductAuthStatusBody {
            service_auth_enabled: true,
            service_auth_scoped: false,
            service_auth_legacy: true,
            service_auth_scopes: vec!["runs:read".into()],
            ..ProductAuthStatusBody::default()
        };

        let issue = dispatch_service_token_issue("SignalHive", &action, &auth_status)
            .expect("legacy token should be rejected");
        assert_eq!(issue.0, "service_token_rotation_required");
    }

    #[test]
    fn dispatch_service_token_issue_blocks_expired_machine_tokens() {
        let action = contract::ProductAction {
            id: "scan".into(),
            label: "Run signal scan".into(),
            method: "POST".into(),
            path: "/scan".into(),
            description: String::new(),
            starts_run: true,
            destructive: false,
            required_scopes: vec!["actions:dispatch".into()],
        };
        let auth_status = ProductAuthStatusBody {
            service_auth_enabled: true,
            service_auth_scoped: true,
            service_auth_expired: true,
            service_auth_scopes: vec!["actions:dispatch".into()],
            ..ProductAuthStatusBody::default()
        };

        let issue = dispatch_service_token_issue("SignalHive", &action, &auth_status)
            .expect("expired token should be rejected");
        assert_eq!(issue.0, "service_token_expired");
    }
}
