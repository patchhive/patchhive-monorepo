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
