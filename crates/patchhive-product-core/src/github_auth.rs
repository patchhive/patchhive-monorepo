use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT},
    Client,
};
use serde::Deserialize;

const GITHUB_API_USER: &str = "https://api.github.com/user";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GitHubIdentity {
    pub login: String,
}

pub fn github_token() -> Option<String> {
    ["BOT_GITHUB_TOKEN", "GITHUB_TOKEN"]
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn github_token_configured() -> bool {
    github_token().is_some()
}

/// Returns whether a token represents GitHub App authentication that may create check runs.
///
/// GitHub rejects check-run creation from personal access tokens even when their repository
/// scopes permit commit statuses and issue comments. Installation and user access tokens issued
/// to GitHub Apps use the `ghs_` and `ghu_` prefixes respectively. Unknown token formats take the
/// conservative commit-status path instead of producing a predictable authorization failure.
pub fn github_token_may_create_check_runs(token: &str) -> bool {
    let token = token.trim();
    token.starts_with("ghs_") || token.starts_with("ghu_")
}

pub async fn verify_github_token(client: &Client) -> Result<GitHubIdentity> {
    let token = github_token()
        .ok_or_else(|| anyhow!("[missing_token]: BOT_GITHUB_TOKEN or GITHUB_TOKEN is not set"))?;
    let response = client
        .get(GITHUB_API_USER)
        .headers(github_headers(&token)?)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .context("GitHub token verification request failed")?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!(
            "GitHub rejected token verification with HTTP {}",
            status.as_u16()
        ));
    }
    let identity = response
        .json::<GitHubIdentity>()
        .await
        .context("GitHub token verification returned an invalid identity response")?;
    if identity.login.trim().is_empty() {
        return Err(anyhow!("GitHub token verification returned an empty login"));
    }
    Ok(identity)
}

fn github_headers(token: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("patchhive-token-verifier/0.1"),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}"))
            .context("GitHub token contains invalid header characters")?,
    );
    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_requires_a_login_field() {
        let identity: GitHubIdentity = serde_json::from_str(r#"{"login":"patchhive"}"#).unwrap();
        assert_eq!(identity.login, "patchhive");
    }

    #[test]
    fn headers_reject_control_characters() {
        assert!(github_headers("token\nvalue").is_err());
    }

    #[test]
    fn only_github_app_tokens_attempt_check_runs() {
        assert!(github_token_may_create_check_runs("ghs_installation"));
        assert!(github_token_may_create_check_runs("ghu_user_access"));
        assert!(!github_token_may_create_check_runs("ghp_classic_pat"));
        assert!(!github_token_may_create_check_runs(
            "github_pat_fine_grained"
        ));
        assert!(!github_token_may_create_check_runs("unrecognized-token"));
    }
}
