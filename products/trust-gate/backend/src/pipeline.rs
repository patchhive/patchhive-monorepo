// pipeline.rs — TrustGate pipeline (modular)

mod failguard;
mod github;
mod review;
mod routes;
mod rules;
pub mod types;

pub use routes::{
    capabilities, github_webhook, history, history_detail, review, review_github_pr,
    rule_packs, runs, unique_repos,
};
