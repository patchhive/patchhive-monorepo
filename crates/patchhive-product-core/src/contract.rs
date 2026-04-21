use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CONTRACT_SCHEMA_VERSION: &str = "patchhive.product.contract.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductCapabilities {
    pub schema_version: String,
    pub product_slug: String,
    pub display_name: String,
    pub version: String,
    pub standalone: bool,
    pub hivecore: HiveCoreLifecycleSupport,
    pub routes: ProductContractRoutes,
    pub actions: Vec<ProductAction>,
    pub links: Vec<ProductLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HiveCoreLifecycleSupport {
    pub can_launch: bool,
    pub can_start_runs: bool,
    pub can_list_runs: bool,
    pub can_read_run_detail: bool,
    pub can_apply_settings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductContractRoutes {
    pub health: String,
    pub startup_checks: String,
    pub capabilities: String,
    pub runs: String,
    pub run_detail_template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings_apply: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductAction {
    pub id: String,
    pub label: String,
    pub method: String,
    pub path: String,
    pub description: String,
    pub starts_run: bool,
    pub destructive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductLink {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductRunsResponse {
    pub schema_version: String,
    pub product_slug: String,
    pub runs: Vec<ProductRunSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductRunSummary {
    pub id: String,
    pub status: String,
    pub title: String,
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
    pub detail_path: String,
    pub raw: Value,
}

pub fn capabilities(
    product_slug: impl Into<String>,
    display_name: impl Into<String>,
    actions: Vec<ProductAction>,
    links: Vec<ProductLink>,
) -> ProductCapabilities {
    let can_start_runs = actions.iter().any(|action| action.starts_run);
    ProductCapabilities {
        schema_version: CONTRACT_SCHEMA_VERSION.into(),
        product_slug: product_slug.into(),
        display_name: display_name.into(),
        version: "0.1.0".into(),
        standalone: true,
        hivecore: HiveCoreLifecycleSupport {
            can_launch: true,
            can_start_runs,
            can_list_runs: true,
            can_read_run_detail: true,
            can_apply_settings: false,
        },
        routes: ProductContractRoutes {
            health: "/health".into(),
            startup_checks: "/startup/checks".into(),
            capabilities: "/capabilities".into(),
            runs: "/runs".into(),
            run_detail_template: "/runs/{id}".into(),
            settings_apply: None,
        },
        actions,
        links,
    }
}

pub fn action(
    id: impl Into<String>,
    label: impl Into<String>,
    method: impl Into<String>,
    path: impl Into<String>,
    description: impl Into<String>,
    starts_run: bool,
) -> ProductAction {
    ProductAction {
        id: id.into(),
        label: label.into(),
        method: method.into(),
        path: path.into(),
        description: description.into(),
        starts_run,
        destructive: false,
    }
}

pub fn link(
    id: impl Into<String>,
    label: impl Into<String>,
    path: impl Into<String>,
) -> ProductLink {
    ProductLink {
        id: id.into(),
        label: label.into(),
        path: path.into(),
    }
}

pub fn runs_from_history<T: Serialize>(
    product_slug: impl Into<String>,
    history_items: Vec<T>,
) -> ProductRunsResponse {
    let product_slug = product_slug.into();
    let runs = history_items
        .into_iter()
        .filter_map(|item| serde_json::to_value(item).ok())
        .map(|raw| run_summary_from_value(&raw))
        .collect();

    ProductRunsResponse {
        schema_version: CONTRACT_SCHEMA_VERSION.into(),
        product_slug,
        runs,
    }
}

pub fn runs_from_values(
    product_slug: impl Into<String>,
    history_items: Vec<Value>,
) -> ProductRunsResponse {
    let product_slug = product_slug.into();
    let runs = history_items
        .into_iter()
        .map(|raw| run_summary_from_value(&raw))
        .collect();

    ProductRunsResponse {
        schema_version: CONTRACT_SCHEMA_VERSION.into(),
        product_slug,
        runs,
    }
}

fn run_summary_from_value(raw: &Value) -> ProductRunSummary {
    let id = first_string(raw, &["id", "run_id", "scan_id", "review_id"])
        .unwrap_or_else(|| "unknown".into());
    let status = first_string(raw, &["status", "recommendation", "readiness"])
        .unwrap_or_else(|| "completed".into());
    let created_at =
        first_string(raw, &["created_at", "started_at", "opened_at"]).unwrap_or_default();
    let updated_at =
        first_string(raw, &["updated_at", "finished_at", "last_checked"]).unwrap_or_default();
    let title = first_string(
        raw,
        &[
            "title",
            "repo",
            "repo_name",
            "repository",
            "target_repo",
            "pr_url",
            "top_repo",
        ],
    )
    .unwrap_or_else(|| id.clone());
    let summary = first_string(raw, &["summary", "message", "decision", "reason"])
        .or_else(|| numeric_summary(raw))
        .unwrap_or_default();

    ProductRunSummary {
        id: id.clone(),
        status,
        title,
        summary,
        created_at,
        updated_at,
        detail_path: format!("/runs/{id}"),
        raw: raw.clone(),
    }
}

fn first_string(raw: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        raw.get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn numeric_summary(raw: &Value) -> Option<String> {
    let object = raw.as_object()?;
    let parts = object
        .iter()
        .filter_map(|(key, value)| {
            if matches!(
                key.as_str(),
                "id" | "created_at" | "started_at" | "updated_at"
            ) {
                return None;
            }
            value
                .as_u64()
                .map(|number| format!("{} {}", number, key.replace('_', " ")))
        })
        .take(3)
        .collect::<Vec<_>>();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

#[cfg(test)]
mod tests {
    use super::{action, capabilities, runs_from_values};
    use serde_json::json;

    #[test]
    fn capabilities_report_run_support_from_actions() {
        let caps = capabilities(
            "signal-hive",
            "SignalHive",
            vec![action(
                "scan",
                "Run scan",
                "POST",
                "/scan",
                "Scan repos",
                true,
            )],
            vec![],
        );

        assert_eq!(caps.schema_version, "patchhive.product.contract.v1");
        assert!(caps.standalone);
        assert!(caps.hivecore.can_start_runs);
        assert_eq!(caps.routes.runs, "/runs");
    }

    #[test]
    fn runs_from_history_values_normalizes_common_fields() {
        let runs = runs_from_values(
            "dep-triage",
            vec![json!({
                "id": "scan_123",
                "repo": "patchhive/example",
                "created_at": "2026-04-21T10:00:00Z",
                "summary": "2 updates need attention",
                "tracked_items": 7
            })],
        );

        assert_eq!(runs.product_slug, "dep-triage");
        assert_eq!(runs.runs[0].id, "scan_123");
        assert_eq!(runs.runs[0].status, "completed");
        assert_eq!(runs.runs[0].title, "patchhive/example");
        assert_eq!(runs.runs[0].detail_path, "/runs/scan_123");
    }
}
