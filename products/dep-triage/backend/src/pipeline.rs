pub mod analysis;
pub mod routes;
pub mod scoring;
pub mod utils;

pub use routes::{
    auth_status, capabilities, gen_key, gen_service_token, health, history, history_detail,
    login, overview, rotate_service_token, runs, scan_github_dependencies, startup_checks_route,
};

#[cfg(test)]
mod tests {
    use super::{
        utils::{compare_versions, infer_runtime_impact, parse_package_name},
        analysis::looks_like_dependency_pr,
    };
    use crate::github::{GitHubPullFile, GitHubPullRequest};

    #[test]
    fn compares_semver_jumps() {
        assert_eq!(compare_versions("1.2.3", "2.0.0").as_deref(), Some("major"));
        assert_eq!(compare_versions("1.2.3", "1.3.0").as_deref(), Some("minor"));
        assert_eq!(compare_versions("1.2.3", "1.2.4").as_deref(), Some("patch"));
    }

    #[test]
    fn parses_dependabot_package_name() {
        assert_eq!(
            parse_package_name("build(deps): bump reqwest from 0.11.0 to 0.12.0").as_deref(),
            Some("reqwest")
        );
        assert_eq!(
            parse_package_name("chore: update dependency react to v19").as_deref(),
            Some("react")
        );
    }

    #[test]
    fn detects_dependency_prs_from_titles_and_files() {
        let pr = GitHubPullRequest {
            title: "build(deps): bump tokio from 1.42 to 1.43".into(),
            ..GitHubPullRequest::default()
        };
        let files = vec![GitHubPullFile {
            filename: "backend/Cargo.toml".into(),
            ..GitHubPullFile::default()
        }];
        assert!(looks_like_dependency_pr(&pr, &files));

        let runtime = infer_runtime_impact(
            &[String::from("backend/Cargo.toml")],
            &pr.title,
            None,
            "tokio",
        );
        assert_eq!(runtime, "runtime");
    }
}
