use std::collections::HashSet;

use axum::{http::StatusCode, Json};
use chrono::{Duration, Utc};
use patchhive_product_core::scope_policy::normalize_repo_name;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    db,
    models::{
        now_rfc3339, ok, PrBudgetLimitingLayer, PrBudgetReservation, PrBudgetStatusResponse,
        PrBudgetUsage, PrReservationDecision, PrReservationDenial, PrReservationRequest,
        PrReservationState, PrRunReleaseRequest, ProductPrBudget, RepositoryPoliciesResponse,
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
            // Trust is an elevation, never a way around an exclusion, so an entry
            // that is both excluded and trusted keeps only the exclusion.
            trusted: input.trusted && !input.operator_excluded,
            operator_excluded: input.operator_excluded,
            allowlisted: input.allowlisted && !input.operator_excluded,
            // Verified public opt-outs are preserved by the store, not by this
            // request: an operator edit cannot set or clear one.
            public_opt_out: false,
            source: "operator".into(),
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

/// Evaluate one repository against the shared suite-wide policy store.
///
/// The logic used to live here, reading HiveCore's own table *and* two free-text
/// settings fields. Four other products carried their own copies of the same idea.
/// One evaluator over five stores looks consistent and is not — the failure mode is
/// a repository denied in one product and reachable from another.
///
/// So this now answers from `patchhive_product_core::repo_policy` and nothing else.
/// HiveCore keeps the editing surface and stays the operator's single place to say
/// "stay off this repository"; it no longer keeps a private opinion about it.
pub(super) fn evaluate_repository_policy(
    request: &RepositoryPolicyDecisionRequest,
) -> InternalApiResult<RepositoryPolicyDecision> {
    let product = request.product.trim().to_ascii_lowercase();
    let operation = request.operation.trim().to_ascii_lowercase();
    if product.is_empty() || operation.is_empty() {
        return Err(Box::new(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_policy_request",
            "Repository policy checks require product and operation.",
        )));
    }
    // Reject a malformed name before evaluating. The shared evaluator also refuses
    // it, but as a denial rather than a validation error, and a caller that
    // mistyped a repository deserves the difference.
    if normalize_repo_name(&request.repository).is_none() {
        return Err(Box::new(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_repository",
            "Repository must use owner/repo format.",
        )));
    }

    let decision = db::evaluate_repository_policy(&request.repository, &product, &operation)
        .map_err(|err| {
            Box::new(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "repository_policy_read_failed",
                format!("HiveCore could not evaluate repository policy: {err}"),
            ))
        })?;

    let listed = db::repository_policy_result(&decision.repository)
        .ok()
        .flatten()
        .unwrap_or_default();

    Ok(RepositoryPolicyDecision {
        repository: decision.repository,
        product: decision.product,
        operation: decision.operation,
        decision: if decision.allowed {
            "allowed"
        } else {
            "blocked"
        }
        .into(),
        reason: decision.reason,
        trusted: decision.trusted,
        operator_excluded: listed.operator_excluded,
        // The store is consulted every time now, so this is genuinely checked
        // rather than a field that was always false.
        public_opt_out_checked: true,
        public_opted_out: listed.public_opt_out,
        chain: decision.chain,
        policy_version: decision.policy_version.to_string(),
        evaluated_at: decision.evaluated_at,
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
) -> ApiResult<PrReservationDecision> {
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
        return Ok(Json(ok(PrReservationDecision::Denied {
            denial: PrReservationDenial {
                reason: policy.reason,
                limiting_layer: PrBudgetLimitingLayer::RepositoryPolicy,
                usage: current_pr_budget_usage(&product).map_err(|error| *error)?,
            },
        })));
    }

    let now = Utc::now();
    let reservation = PrBudgetReservation {
        id: format!("prr_{}", Uuid::now_v7()),
        product,
        repository,
        run_id: run_id.into(),
        action: action.into(),
        lifecycle: PrReservationState::Reserved {
            expires_at: (now + Duration::minutes(10)).to_rfc3339(),
        },
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    };
    let attempt = db::reserve_pr_slot(&reservation).map_err(|err| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pr_reservation_failed",
            format!("HiveCore could not reserve PR capacity: {err}"),
        )
    })?;
    Ok(Json(ok(attempt)))
}

pub(super) async fn commit_pr_budget_reservation(
    id: String,
    pr_url: String,
) -> ApiResult<PrBudgetReservation> {
    let pr_url = pr_url.trim();
    let reservation = db::pr_budget_reservation(&id)
        .map_err(|err| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "pr_reservation_read_failed",
                format!("HiveCore could not read the PR reservation: {err}"),
            )
        })?
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "pr_reservation_not_found",
                "PR reservation not found or no longer active.",
            )
        })?;
    if !github_pull_request_url_matches_repository(pr_url, &reservation.repository) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_pr_url",
            "Committed reservations require a GitHub pull-request URL for the reserved repository.",
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
    if !reservation.lifecycle.is_committed() {
        return Err(api_error(
            StatusCode::CONFLICT,
            "pr_reservation_not_active",
            format!(
                "PR reservation cannot be committed from status '{}'.",
                reservation.lifecycle.label()
            ),
        ));
    }
    Ok(Json(ok(reservation)))
}

fn github_pull_request_url_matches_repository(pr_url: &str, repository: &str) -> bool {
    let expected = format!("https://github.com/{}/pull/", repository.trim_matches('/'));
    let Some(number) = pr_url.strip_prefix(&expected) else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
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
    if !reservation.lifecycle.is_released() {
        return Err(api_error(
            StatusCode::CONFLICT,
            "pr_reservation_not_active",
            format!(
                "PR reservation cannot be released from status '{}'.",
                reservation.lifecycle.label()
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
    let suite_limit = db::suite_pr_limit().map_err(pr_budget_read_error)?;
    let configured = db::product_pr_limits().map_err(pr_budget_read_error)?;
    let reservations = db::pr_budget_reservations(50).map_err(pr_budget_read_error)?;
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

fn current_pr_budget_usage(product: &str) -> InternalApiResult<PrBudgetUsage> {
    let product_limit = db::product_pr_limits()
        .map_err(pr_budget_read_error)?
        .get(product)
        .copied()
        .unwrap_or_else(|| db::default_product_pr_limit(product));
    let suite_limit = db::suite_pr_limit().map_err(pr_budget_read_error)?;
    let (suite_used, product_usage) = db::active_pr_usage().map_err(|err| {
        Box::new(api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pr_budget_status_failed",
            format!("HiveCore could not calculate active PR usage: {err}"),
        ))
    })?;
    Ok(PrBudgetUsage {
        product_limit,
        product_used: product_usage.get(product).copied().unwrap_or(0),
        suite_limit,
        suite_used,
    })
}

fn pr_budget_read_error(err: rusqlite::Error) -> InternalApiError {
    Box::new(api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "pr_budget_status_failed",
        format!("HiveCore could not read PR budget state: {err}"),
    ))
}

#[cfg(test)]
mod tests {
    use patchhive_product_core::repo_policy::operation_requires_trust;

    #[test]
    fn only_elevated_operations_require_repository_trust() {
        // Kept as a HiveCore-side assertion because HiveCore is where the operator
        // grants trust: if the shared list of trust-gated operations ever changes,
        // the control plane's promise about what trust unlocks changes with it.
        assert!(operation_requires_trust("execute_repository_tests"));
        assert!(!operation_requires_trust("open_pull_request"));
    }
}
