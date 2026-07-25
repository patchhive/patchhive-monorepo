use anyhow::Result;

use crate::config::Config;

pub async fn init_enabled_products(config: &Config) -> Result<()> {
    if config.product_selection.enables("merge-keeper") {
        merge_keeper::init_runtime().await?;
    }
    if config.product_selection.enables("release-sentry") {
        release_sentry::init_runtime().await?;
    }
    if config.product_selection.enables("dep-triage") {
        dep_triage::init_runtime().await?;
    }
    if config.product_selection.enables("vuln-triage") {
        vuln_triage::init_runtime().await?;
    }
    if config.product_selection.enables("flake-sting") {
        flake_sting::init_runtime().await?;
    }
    if config.product_selection.enables("review-bee") {
        review_bee::init_runtime().await?;
    }
    if config.product_selection.enables("trust-gate") {
        trust_gate::init_runtime().await?;
    }
    if config.product_selection.enables("repo-memory") {
        repo_memory::init_runtime().await?;
    }
    if config.product_selection.enables("signal-hive") {
        signal_hive::init_runtime().await?;
    }
    if config.product_selection.enables("refactor-scout") {
        refactor_scout::init_runtime().await?;
    }
    if config.product_selection.enables("repo-reaper") {
        repo_reaper::init_runtime().await?;
    }
    Ok(())
}

pub fn merge_keeper_router() -> axum::Router {
    merge_keeper::router()
}

pub fn release_sentry_router() -> axum::Router {
    release_sentry::router()
}

pub fn dep_triage_router() -> axum::Router {
    dep_triage::router()
}

pub fn vuln_triage_router() -> axum::Router {
    vuln_triage::router()
}

pub fn flake_sting_router() -> axum::Router {
    flake_sting::router()
}

pub fn review_bee_router() -> axum::Router {
    review_bee::router()
}

pub fn trust_gate_router() -> axum::Router {
    trust_gate::router()
}

pub fn repo_memory_router() -> axum::Router {
    repo_memory::router()
}

pub fn signal_hive_router() -> axum::Router {
    signal_hive::router()
}

pub fn refactor_scout_router() -> axum::Router {
    refactor_scout::router()
}

pub fn repo_reaper_router() -> axum::Router {
    repo_reaper::router()
}

/// Auth posture for one mounted product engine.
///
/// Read in-process. HiveCore previously fanned out one `/auth/status` request per
/// product from the browser, which is both N calls per refresh and enough traffic to
/// trip the sensitive-route rate limiter. Every engine is compiled into this binary,
/// so the same information is a direct call — no HTTP, no rate limit, one response.
#[derive(serde::Serialize)]
pub struct ProductAuthStatus {
    pub key: &'static str,
    /// None when the product is not enabled in this runtime.
    pub status: Option<serde_json::Value>,
}

pub fn auth_statuses(config: &Config) -> Vec<ProductAuthStatus> {
    macro_rules! status_for {
        ($key:literal, $module:ident) => {
            ProductAuthStatus {
                key: $key,
                status: if config.product_selection.enables($key) {
                    Some($module::auth::auth_status_payload())
                } else {
                    None
                },
            }
        };
    }

    vec![
        status_for!("merge-keeper", merge_keeper),
        status_for!("release-sentry", release_sentry),
        status_for!("dep-triage", dep_triage),
        status_for!("vuln-triage", vuln_triage),
        status_for!("flake-sting", flake_sting),
        status_for!("review-bee", review_bee),
        status_for!("trust-gate", trust_gate),
        status_for!("repo-memory", repo_memory),
        status_for!("signal-hive", signal_hive),
        status_for!("refactor-scout", refactor_scout),
        status_for!("repo-reaper", repo_reaper),
    ]
}

/// Outcome of a provisioning attempt. The minted token is deliberately absent:
/// it is written to the product's own storage and never travels to the caller.
pub enum ProvisionOutcome {
    /// New posture after the mint or rotation.
    Provisioned(serde_json::Value),
    /// The product's own guard refused this caller.
    Forbidden,
    /// The engine is not enabled in this runtime.
    NotEnabled,
    /// No engine is registered under that key.
    Unknown,
    Failed(String),
}

/// Mint or rotate a product's service token, in-process.
///
/// Authorization is delegated to the product's own auth module rather than
/// re-implemented here, so the suite route cannot be a softer door than the
/// product's own `/auth/generate-service-token`. Same guard, same answer.
pub fn provision_service_token(
    config: &Config,
    key: &str,
    headers: &axum::http::HeaderMap,
    peer: Option<std::net::SocketAddr>,
    rotate: bool,
) -> ProvisionOutcome {
    macro_rules! provision_with {
        ($module:ident) => {{
            let allowed = if rotate {
                $module::auth::service_token_rotation_allowed_from_peer(headers, peer)
            } else {
                $module::auth::service_token_generation_allowed_from_peer(headers, peer)
            };
            if !allowed {
                return ProvisionOutcome::Forbidden;
            }

            // Rotation is the correct verb once a token exists; minting again would
            // leave the previous one valid.
            let should_rotate = rotate || $module::auth::service_auth_enabled();
            let result = if should_rotate {
                $module::auth::rotate_and_save_service_token()
            } else {
                $module::auth::generate_and_save_service_token()
            };

            match result {
                // The token itself is dropped here on purpose.
                Ok(_) => ProvisionOutcome::Provisioned($module::auth::auth_status_payload()),
                Err(error) => ProvisionOutcome::Failed(error.to_string()),
            }
        }};
    }

    if !config.product_selection.enables(key) {
        return match key {
            "merge-keeper" | "release-sentry" | "dep-triage" | "vuln-triage" | "flake-sting"
            | "review-bee" | "trust-gate" | "repo-memory" | "signal-hive" | "refactor-scout"
            | "repo-reaper" => ProvisionOutcome::NotEnabled,
            _ => ProvisionOutcome::Unknown,
        };
    }

    match key {
        "merge-keeper" => provision_with!(merge_keeper),
        "release-sentry" => provision_with!(release_sentry),
        "dep-triage" => provision_with!(dep_triage),
        "vuln-triage" => provision_with!(vuln_triage),
        "flake-sting" => provision_with!(flake_sting),
        "review-bee" => provision_with!(review_bee),
        "trust-gate" => provision_with!(trust_gate),
        "repo-memory" => provision_with!(repo_memory),
        "signal-hive" => provision_with!(signal_hive),
        "refactor-scout" => provision_with!(refactor_scout),
        "repo-reaper" => provision_with!(repo_reaper),
        _ => ProvisionOutcome::Unknown,
    }
}
