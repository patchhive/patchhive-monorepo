use std::{error::Error, fmt};

use reqwest::StatusCode;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitHubApiErrorKind {
    InvalidToken,
    MissingTokenScope,
    FeatureDisabled,
    NotFoundOrInaccessible,
    RateLimited,
    Forbidden,
    HttpStatus,
}

impl GitHubApiErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidToken => "invalid_token",
            Self::MissingTokenScope => "missing_token_scope",
            Self::FeatureDisabled => "feature_disabled",
            Self::NotFoundOrInaccessible => "not_found_or_inaccessible",
            Self::RateLimited => "rate_limited",
            Self::Forbidden => "forbidden",
            Self::HttpStatus => "http_status",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubApiError {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub status_label: String,
    pub kind: GitHubApiErrorKind,
    pub message: String,
    pub body_preview: String,
}

impl GitHubApiError {
    pub fn from_response(
        method: impl Into<String>,
        path: impl Into<String>,
        status: StatusCode,
        body: &str,
    ) -> Self {
        let message = if status == StatusCode::BAD_GATEWAY
            || status == StatusCode::SERVICE_UNAVAILABLE
            || status == StatusCode::GATEWAY_TIMEOUT
        {
            "GitHub is temporarily unavailable after retrying the request. Try the scan again shortly."
                .to_string()
        } else {
            github_message_from_body(body)
        };
        let kind = classify_github_api_error(status, &message, body);
        Self {
            method: method.into(),
            path: path.into(),
            status: status.as_u16(),
            status_label: status
                .canonical_reason()
                .unwrap_or("GitHub API error")
                .to_string(),
            kind,
            message,
            body_preview: response_preview(body),
        }
    }
}

impl fmt::Display for GitHubApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = if self.message.trim().is_empty() {
            self.body_preview.as_str()
        } else {
            self.message.as_str()
        };
        write!(
            f,
            "GitHub {} {} -> {} {} [{}]: {}",
            self.method,
            self.path,
            self.status,
            self.status_label,
            self.kind.as_str(),
            message
        )
    }
}

impl Error for GitHubApiError {}

pub fn classify_github_api_error(
    status: StatusCode,
    message: &str,
    body: &str,
) -> GitHubApiErrorKind {
    let text = format!("{message}\n{body}").to_ascii_lowercase();
    if status == StatusCode::UNAUTHORIZED {
        return GitHubApiErrorKind::InvalidToken;
    }
    if status == StatusCode::NOT_FOUND {
        return GitHubApiErrorKind::NotFoundOrInaccessible;
    }
    if status == StatusCode::FORBIDDEN {
        if text.contains("dependabot alerts are disabled")
            || text.contains("code scanning is not enabled")
            || text.contains("advanced security must be enabled")
        {
            return GitHubApiErrorKind::FeatureDisabled;
        }
        if text.contains("resource not accessible by personal access token")
            || text.contains("requires authentication")
            || text.contains("must have")
            || text.contains("missing")
            || text.contains("scope")
        {
            return GitHubApiErrorKind::MissingTokenScope;
        }
        if text.contains("rate limit") || text.contains("secondary rate limit") {
            return GitHubApiErrorKind::RateLimited;
        }
        return GitHubApiErrorKind::Forbidden;
    }
    if status == StatusCode::TOO_MANY_REQUESTS || text.contains("rate limit") {
        return GitHubApiErrorKind::RateLimited;
    }
    GitHubApiErrorKind::HttpStatus
}

pub fn github_error_is_permission_blocked(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    message.contains("[missing_token_scope]")
        || message.contains("[forbidden]")
        || lower.contains("403 forbidden")
        || lower.contains("resource not accessible")
}

pub fn github_error_is_feature_disabled(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    message.contains("[feature_disabled]")
        || lower.contains("dependabot alerts are disabled")
        || lower.contains("code scanning is not enabled")
}

pub fn github_error_is_token_missing(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    message.contains("[missing_token]")
        || lower.contains("bot_github_token is not set")
        || lower.contains("github_token is not set")
}

pub fn github_error_is_token_invalid(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    message.contains("[invalid_token]") || lower.contains("bad credentials")
}

pub fn github_message_from_body(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(str::trim)
                .map(str::to_string)
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| response_preview(body))
}

pub fn response_preview(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = compact.chars().take(240).collect::<String>();
    if compact.chars().count() > 240 {
        preview.push_str("...");
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_dependabot_disabled_as_feature_disabled() {
        let err = GitHubApiError::from_response(
            "GET",
            "/repos/o/r/dependabot/alerts",
            StatusCode::FORBIDDEN,
            r#"{"message":"Dependabot alerts are disabled for this repository."}"#,
        );

        assert_eq!(err.kind, GitHubApiErrorKind::FeatureDisabled);
        assert!(err.to_string().contains("[feature_disabled]"));
    }

    #[test]
    fn classifies_pat_scope_errors_as_missing_scope() {
        let err = GitHubApiError::from_response(
            "GET",
            "/repos/o/r/code-scanning/alerts",
            StatusCode::FORBIDDEN,
            r#"{"message":"Resource not accessible by personal access token"}"#,
        );

        assert_eq!(err.kind, GitHubApiErrorKind::MissingTokenScope);
        assert!(github_error_is_permission_blocked(&err.to_string()));
    }

    #[test]
    fn classifies_unauthorized_as_invalid_token() {
        let err = GitHubApiError::from_response(
            "GET",
            "/rate_limit",
            StatusCode::UNAUTHORIZED,
            r#"{"message":"Bad credentials"}"#,
        );

        assert_eq!(err.kind, GitHubApiErrorKind::InvalidToken);
        assert_eq!(err.message, "Bad credentials");
        assert!(github_error_is_token_invalid(&err.to_string()));
    }

    #[test]
    fn hides_html_for_transient_github_outages() {
        let err = GitHubApiError::from_response(
            "GET",
            "/search/repositories",
            StatusCode::SERVICE_UNAVAILABLE,
            "<!DOCTYPE html><html><body>temporary outage</body></html>",
        );

        assert_eq!(err.kind, GitHubApiErrorKind::HttpStatus);
        assert!(err.to_string().contains("temporarily unavailable"));
        assert!(!err.to_string().contains("DOCTYPE"));
    }
}
