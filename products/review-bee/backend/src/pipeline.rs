mod analysis;
mod review;
mod routes;

// Re-export all public route handlers for main.rs.
// External callers use `pipeline::function_name` — this keeps them working.
pub use routes::{
    auth_status, capabilities, gen_key, gen_service_token, github_webhook, health, history,
    history_detail, login, overview, review_github_pr, rotate_service_token, runs,
    startup_checks_route,
};

// Exposed for tests only.
#[cfg(test)]
use routes::supported_webhook_action;

// Exposed for route handlers that need internal types.
#[allow(unused)]
pub(crate) use routes::ApiError;
#[allow(unused)]
pub(crate) use routes::JsonResult;
#[allow(unused)]
pub(crate) use routes::LoginBody;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actionability_filters_praise_but_keeps_requests() {
        assert!(!analysis::actionable_text("LGTM, nice work."));
        assert!(analysis::actionable_text(
            "Could you add a regression test for this path?"
        ));
    }

    #[test]
    fn path_bucket_keeps_useful_area_context() {
        assert_eq!(
            analysis::path_bucket("src/reaper/fix_worker.rs"),
            "src/reaper"
        );
        assert_eq!(analysis::path_bucket("docs/guide.md"), "docs");
    }

    #[test]
    fn webhook_support_matrix_stays_intentional() {
        assert!(supported_webhook_action("pull_request", "synchronize"));
        assert!(supported_webhook_action("pull_request_review", "submitted"));
        assert!(supported_webhook_action(
            "pull_request_review_comment",
            "created"
        ));
        assert!(supported_webhook_action(
            "pull_request_review_thread",
            "resolved"
        ));
        assert!(!supported_webhook_action("issues", "opened"));
        assert!(!supported_webhook_action("pull_request", "closed"));
    }
}
