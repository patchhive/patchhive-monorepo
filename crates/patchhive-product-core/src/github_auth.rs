use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT},
    Client,
};
use serde::Deserialize;
use std::fmt;

const GITHUB_API_USER: &str = "https://api.github.com/user";
pub const PATCHHIVE_GITHUB_TOKEN_RO: &str = "PATCHHIVE_GITHUB_TOKEN_RO";
pub const REPO_REAPER_GITHUB_TOKEN_RW: &str = "REPO_REAPER_GITHUB_TOKEN_RW";
const LEGACY_BOT_GITHUB_TOKEN: &str = "BOT_GITHUB_TOKEN";
const LEGACY_GITHUB_TOKEN: &str = "GITHUB_TOKEN";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GitHubIdentity {
    pub login: String,
}

#[derive(Clone, Eq, PartialEq)]
pub struct ResolvedGitHubToken {
    value: String,
    env_var: &'static str,
    legacy: bool,
}

impl fmt::Debug for ResolvedGitHubToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedGitHubToken")
            .field("value", &"[redacted]")
            .field("env_var", &self.env_var)
            .field("legacy", &self.legacy)
            .finish()
    }
}

impl ResolvedGitHubToken {
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn into_value(self) -> String {
        self.value
    }

    pub fn env_var(&self) -> &'static str {
        self.env_var
    }

    pub fn is_legacy(&self) -> bool {
        self.legacy
    }
}

pub fn resolved_github_read_token() -> Option<ResolvedGitHubToken> {
    resolve_from_env(&[
        (PATCHHIVE_GITHUB_TOKEN_RO, false),
        (LEGACY_BOT_GITHUB_TOKEN, true),
        (LEGACY_GITHUB_TOKEN, true),
    ])
}

pub fn github_read_token() -> Option<String> {
    resolved_github_read_token().map(ResolvedGitHubToken::into_value)
}

/// Compatibility alias for read-only GitHub clients.
pub fn github_token() -> Option<String> {
    github_read_token()
}

pub fn github_token_configured() -> bool {
    resolved_github_read_token().is_some()
}

pub fn github_write_token(env_var: &'static str) -> Option<String> {
    resolve_from_env(&[(env_var, false)]).map(ResolvedGitHubToken::into_value)
}

pub fn github_write_token_configured(env_var: &'static str) -> bool {
    github_write_token(env_var).is_some()
}

pub fn github_read_token_source() -> Option<&'static str> {
    resolved_github_read_token().map(|token| token.env_var())
}

pub fn github_read_token_uses_legacy_name() -> bool {
    resolved_github_read_token().is_some_and(|token| token.is_legacy())
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
    let token = github_read_token()
        .ok_or_else(|| anyhow!("[missing_token]: PATCHHIVE_GITHUB_TOKEN_RO is not set"))?;
    verify_github_token_value(client, &token).await
}

pub async fn verify_github_write_token(
    client: &Client,
    env_var: &'static str,
) -> Result<GitHubIdentity> {
    let token = github_write_token(env_var)
        .ok_or_else(|| anyhow!("[missing_token]: {env_var} is not set"))?;
    verify_github_token_value(client, &token).await
}

pub async fn verify_github_token_value(client: &Client, token: &str) -> Result<GitHubIdentity> {
    let response = client
        .get(GITHUB_API_USER)
        .headers(github_headers(token)?)
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

fn resolve_from_env(names: &[(&'static str, bool)]) -> Option<ResolvedGitHubToken> {
    names.iter().find_map(|(name, legacy)| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(|value| ResolvedGitHubToken {
                value,
                env_var: name,
                legacy: *legacy,
            })
    })
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
