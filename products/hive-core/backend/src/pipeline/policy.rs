use std::collections::HashSet;

use axum::{http::StatusCode, Json};
use chrono::{Duration, Utc};
use patchhive_product_core::scope_policy::{normalize_repo_name, RepoScopePolicy};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    db,
    models::{
        now_rfc3339, ok, PrBudgetReservation, PrBudgetStatusResponse, PrReservationRequest,
        PrReservationResponse, PrRunReleaseRequest, ProductPrBudget, RepositoryPoliciesResponse,
        RepositoryPolicy, RepositoryPolicyDecision, RepositoryPolicyDecisionRequest,
        SavePrBudgetRequest, SaveRepositoryPoliciesRequest,
    },
    state::product_catalog,
};

use super::api_error;

type ApiResult<T> = Result<
    Json<crate::models::ApiEnvelope<T>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
>;
type InternalApiError = Box<(StatusCode, Json<crate::models::ApiEnvelope<Value>>)>;
type InternalApiResult<T> = Result<T, InternalApiError>;

pub(super) async fn repository_policies(
) -> Json<crate::models::ApiEnvelope<RepositoryPoliciesResponse>> {
    Json(ok(RepositoryPoliciesResponse {
        policies: db::repository_policies(),
        public_opt_out_available: false,
    }))
}

pub(super) async fn save_repository_policies(
    Json(body): Json<SaveRepositoryPoliciesRequest>,
) -> ApiResult<RepositoryPoliciesResponse> {
    let mut policies = Vec::new();
    let mut seen = HashSet::new();
    for input in body.policies {
        let repository = normalize_repo_name(&input.repository).ok_or_else(|| {
            api_error(
                StatusCode::BAD_REQUEST,
                "invalid_repository",
                format!(
                    "Repository '{}' must use owner/repo format.",
                    input.repository
                ),
            )
        })?;
        if !seen.insert(repository.clone()) {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "duplicate_repository",
                format!("Repository '{repository}' appears more than once."),
            ));
        }
        policies.push(RepositoryPolicy {
            repository,
            trusted: input.trusted && !input.operator_excluded,
            operator_excluded: input.operator_excluded,
            notes: input.notes.trim().to_string(),
            updated_at: now_rfc3339(),
        });
    }
    policies.sort_by(|left, right| left.repository.cmp(&right.repository));
    db::replace_repository_policies(&policies).map_err(|err| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "repository_policy_save_failed",
            format!("HiveCore could not save repository policies: {err}"),
        )
    })?;
    Ok(Json(ok(RepositoryPoliciesResponse {
        policies,
        public_opt_out_available: false,
    })))
}

pub(super) async fn repository_policy_check(
    Json(request): Json<RepositoryPolicyDecisionRequest>,
) -> ApiResult<RepositoryPolicyDecision> {
    let decision = evaluate_repository_policy(&request).map_err(|error| *error)?;
    Ok(Json(ok(decision)))
}

pub(super) fn evaluate_repository_policy(
    request: &RepositoryPolicyDecisionRequest,
) -> InternalApiResult<RepositoryPolicyDecision> {
    let repository = normalize_repo_name(&request.repository).ok_or_else(|| {
        Box::new(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_repository",
            "Repository must use owner/repo format.",
        ))
    })?;
    let product = request.product.trim().to_ascii_lowercase();
    let operation = request.operation.trim().to_ascii_lowercase();
    if product.is_empty() || operation.is_empty() {
        return Err(Box::new(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_policy_request",
            "Repository policy checks require product and operation.",
        )));
    }

    let settings = db::suite_settings();
    let allowlist = parse_repo_set(&settings.repo_allowlist);
    let denylist = parse_repo_set(&settings.repo_denylist);
    let local = db::repository_policy_result(&repository)
        .map_err(|err| {
            Box::new(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "repository_policy_read_failed",
                format!("HiveCore could not evaluate repository policy: {err}"),
            ))
        })?
        .unwrap_or_default();
    let opt_out = if local.operator_excluded {
        HashSet::from([repository.clone()])
    } else {
        HashSet::new()
    };
    let scope = RepoScopePolicy::new(allowlist, denylist, opt_out);
    let scope_decision = scope.decision(&repository);
    let trusted = local.trusted && !local.operator_excluded;
    let requires_trust = operation_requires_trust(&operation);

    let (decision, reason) = if !scope_decision.is_allowed() {
        ("blocked", scope_decision.message(&repository))
    } else if requires_trust && !trusted {
        (
            "blocked",
            format!("Operation '{operation}' requires {repository} to be trusted in HiveCore."),
        )
    } else {
        (
            "allowed",
            if trusted {
                format!("Repository {repository} is eligible and trusted for '{operation}'.")
            } else {
                format!("Repository {repository} is eligible for '{operation}'.")
            },
        )
    };

    Ok(RepositoryPolicyDecision {
        repository,
        product,
        operation,
        decision: decision.into(),
        reason,
        trusted,
        operator_excluded: local.operator_excluded,
        public_opt_out_checked: false,
        public_opted_out: false,
        policy_version: "hivecore.repository-policy.v1".into(),
        evaluated_at: now_rfc3339(),
    })
}

pub(super) async fn pr_budget_status() -> ApiResult<PrBudgetStatusResponse> {
    Ok(Json(ok(build_pr_budget_status().map_err(|error| *error)?)))
}

pub(super) async fn save_pr_budgets(
    Json(body): Json<SavePrBudgetRequest>,
) -> ApiResult<PrBudgetStatusResponse> {
    if body.suite_limit > 1_000 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_suite_pr_limit",
            "Suite PR ceiling must be between 0 and 1000.",
        ));
    }
    let known = product_catalog()
        .iter()
        .map(|product| product.slug)
        .collect::<HashSet<_>>();
    let mut products = Vec::new();
    let mut seen = HashSet::new();
    for input in body.products {
        let product = input.product.trim().to_ascii_lowercase();
        if !known.contains(product.as_str()) {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "invalid_product",
                format!("Unknown product slug '{product}'."),
            ));
        }
        if input.limit > 1_000 {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "invalid_product_pr_limit",
                format!("PR maximum for '{product}' must be between 0 and 1000."),
            ));
        }
        if !seen.insert(product.clone()) {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "duplicate_product",
                format!("Product '{product}' appears more than once."),
            ));
        }
        products.push((product, input.limit));
    }
    db::save_pr_budget_settings(body.suite_limit, &products, &now_rfc3339()).map_err(|err| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pr_budget_save_failed",
            format!("HiveCore could not save PR budgets: {err}"),
        )
    })?;
    Ok(Json(ok(build_pr_budget_status().map_err(|error| *error)?)))
}

pub(super) async fn reserve_pr_budget(
    Json(request): Json<PrReservationRequest>,
) -> ApiResult<PrReservationResponse> {
    let product = request.product.trim().to_ascii_lowercase();
    if !product_catalog()
        .iter()
        .any(|definition| definition.slug == product)
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_product",
            format!("Unknown product slug '{product}'."),
        ));
    }
    let repository = normalize_repo_name(&request.repository).ok_or_else(|| {
        api_error(
            StatusCode::BAD_REQUEST,
            "invalid_repository",
            "Repository must use owner/repo format.",
        )
    })?;
    let run_id = request.run_id.trim();
    let action = request.action.trim();
    if run_id.is_empty() || action.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_reservation",
            "PR reservations require run_id and action.",
        ));
    }

    let policy = evaluate_repository_policy(&RepositoryPolicyDecisionRequest {
        repository: repository.clone(),
        product: product.clone(),
        operation: "open_pull_request".into(),
    })
    .map_err(|error| *error)?;
    if policy.decision != "allowed" {
        return Ok(Json(ok(PrReservationResponse {
            granted: false,
            reason: policy.reason,
            limiting_layer: "repository_policy".into(),
            product_limit: configured_product_limit(&product),
            suite_limit: db::suite_pr_limit(),
            ..PrReservationResponse::default()
        })));
    }

    let now = Utc::now();
    let reservation = PrBudgetReservation {
        id: format!("prr_{}", Uuid::now_v7()),
        product,
        repository,
        run_id: run_id.into(),
        action: action.into(),
        status: "reserved".into(),
        pr_url: String::new(),
        reason: String::new(),
        created_at: now.to_rfc3339(),
        expires_at: (now + Duration::minutes(10)).to_rfc3339(),
        updated_at: now.to_rfc3339(),
    };
    let attempt = db::reserve_pr_slot(&reservation).map_err(|err| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pr_reservation_failed",
            format!("HiveCore could not reserve PR capacity: {err}"),
        )
    })?;
    Ok(Json(ok(PrReservationResponse {
        granted: attempt.granted,
        reason: attempt.reason,
        limiting_layer: attempt.limiting_layer,
        product_limit: attempt.product_limit,
        product_used: attempt.product_used,
        suite_limit: attempt.suite_limit,
        suite_used: attempt.suite_used,
        reservation: attempt.reservation,
    })))
}

pub(super) async fn commit_pr_budget_reservation(
    id: String,
    pr_url: String,
) -> ApiResult<PrBudgetReservation> {
    let pr_url = pr_url.trim();
    if !pr_url.starts_with("https://github.com/") {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_pr_url",
            "Committed reservations require a GitHub pull-request URL.",
        ));
    }
    let reservation = db::commit_pr_reservation(&id, pr_url, &now_rfc3339())
        .map_err(|err| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "pr_reservation_commit_failed",
                format!("HiveCore could not commit the PR reservation: {err}"),
            )
        })?
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "pr_reservation_not_found",
                "PR reservation was not found.",
            )
        })?;
    if reservation.status != "committed" {
        return Err(api_error(
            StatusCode::CONFLICT,
            "pr_reservation_not_active",
            format!(
                "PR reservation cannot be committed from status '{}'.",
                reservation.status
            ),
        ));
    }
    Ok(Json(ok(reservation)))
}

pub(super) async fn release_pr_budget_reservation(
    id: String,
    reason: String,
) -> ApiResult<PrBudgetReservation> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "release_reason_required",
            "Releasing a PR reservation requires a reason.",
        ));
    }
    let reservation = db::release_pr_reservation(&id, reason, &now_rfc3339())
        .map_err(|err| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "pr_reservation_release_failed",
                format!("HiveCore could not release the PR reservation: {err}"),
            )
        })?
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "pr_reservation_not_found",
                "PR reservation was not found.",
            )
        })?;
    if reservation.status != "released" {
        return Err(api_error(
            StatusCode::CONFLICT,
            "pr_reservation_not_active",
            format!(
                "PR reservation cannot be released from status '{}'.",
                reservation.status
            ),
        ));
    }
    Ok(Json(ok(reservation)))
}

pub(super) async fn release_pr_budget_reservations_for_run(
    Json(request): Json<PrRunReleaseRequest>,
) -> ApiResult<Vec<PrBudgetReservation>> {
    let product = request.product.trim().to_ascii_lowercase();
    let run_id = request.run_id.trim();
    let reason = request.reason.trim();
    if !product_catalog()
        .iter()
        .any(|definition| definition.slug == product)
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_product",
            format!("Unknown product slug '{product}'."),
        ));
    }
    if run_id.is_empty() || reason.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_run_release",
            "Run releases require run_id and reason.",
        ));
    }
    let reservations =
        db::release_pr_reservations_for_run(&product, run_id, reason, &now_rfc3339()).map_err(
            |err| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "pr_run_release_failed",
                    format!("HiveCore could not release PR capacity for the run: {err}"),
                )
            },
        )?;
    Ok(Json(ok(reservations)))
}

fn build_pr_budget_status() -> InternalApiResult<PrBudgetStatusResponse> {
    let suite_limit = db::suite_pr_limit();
    let configured = db::product_pr_limits();
    let reservations = db::pr_budget_reservations(50);
    let (suite_used, product_usage) = db::active_pr_usage().map_err(|err| {
        Box::new(api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pr_budget_status_failed",
            format!("HiveCore could not calculate active PR usage: {err}"),
        ))
    })?;
    let products = product_catalog()
        .iter()
        .map(|definition| {
            let limit = configured
                .get(definition.slug)
                .copied()
                .unwrap_or_else(|| db::default_product_pr_limit(definition.slug));
            let used = product_usage.get(definition.slug).copied().unwrap_or(0);
            ProductPrBudget {
                product: definition.slug.into(),
                limit,
                used,
                remaining: limit.saturating_sub(used),
            }
        })
        .collect();
    Ok(PrBudgetStatusResponse {
        suite_limit,
        suite_used,
        suite_remaining: suite_limit.saturating_sub(suite_used),
        products,
        reservations,
    })
}

fn configured_product_limit(product: &str) -> u32 {
    db::product_pr_limits()
        .get(product)
        .copied()
        .unwrap_or_else(|| db::default_product_pr_limit(product))
}

fn parse_repo_set(value: &str) -> HashSet<String> {
    value
        .split([',', ';', '\n', '\r'])
        .filter_map(normalize_repo_name)
        .collect()
}

fn operation_requires_trust(operation: &str) -> bool {
    matches!(
        operation,
        "execute_repository_tests" | "execute_host_tests" | "broader_sandbox"
    )
}

#[cfg(test)]
mod tests {
    use super::{operation_requires_trust, parse_repo_set};

    #[test]
    fn parses_repository_lists_across_supported_separators() {
        let repos = parse_repo_set("Owner/One, owner/two\nowner/three;bad");
        assert_eq!(repos.len(), 3);
        assert!(repos.contains("owner/one"));
    }

    #[test]
    fn only_elevated_operations_require_repository_trust() {
        assert!(operation_requires_trust("execute_repository_tests"));
        assert!(!operation_requires_trust("open_pull_request"));
    }
}
