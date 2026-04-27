// pipeline.rs - Module hub for VulnTriage

use axum::{
    http::StatusCode,
    Json,
};

pub type ApiError = (StatusCode, Json<serde_json::Value>);
pub type JsonResult<T> = Result<Json<T>, ApiError>;

mod analysis;
mod routes;
mod scoring;
mod utils;

pub use routes::{
    capabilities, runs, auth_status, login, gen_key, gen_service_token,
    rotate_service_token, health, startup_checks_route,
    overview, history, history_detail, scan_github_findings,
    api_error,
};

#[cfg(test)]
mod tests {
    use super::scoring::{code_scanning_reachability, owner_hint_for_path, recommend, score_code_scanning};

    #[test]
    fn prefers_public_surface_code_paths() {
        assert_eq!(
            code_scanning_reachability("backend/src/api/auth/routes.rs", &[]),
            "public surface"
        );
        assert_eq!(
            code_scanning_reachability("backend/tests/auth_spec.rs", &["test".into()]),
            "test-only"
        );
    }

    #[test]
    fn owner_hints_follow_paths() {
        assert_eq!(
            owner_hint_for_path("frontend/src/app.jsx"),
            "frontend owners"
        );
        assert_eq!(
            owner_hint_for_path(".github/workflows/ci.yml"),
            "platform / CI owners"
        );
    }

    #[test]
    fn high_scoring_findings_get_fix_now() {
        let score = score_code_scanning("high", "public surface", &[]);
        assert_eq!(recommend(score, "high"), "fix_now");
        assert_eq!(recommend(30, "low"), "watch");
    }
}
