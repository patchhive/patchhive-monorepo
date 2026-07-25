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
