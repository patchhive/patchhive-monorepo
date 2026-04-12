use anyhow::Result;
use patchhive_github_pr::{
    github_token_from_env, GitHubPrClient, GitHubPullRequest, GitHubPullReview,
    GitHubPullReviewThread,
};
use reqwest::Client;

pub struct GitHubReviewContext {
    pub pr: GitHubPullRequest,
    pub reviews: Vec<GitHubPullReview>,
    pub threads: Vec<GitHubPullReviewThread>,
}

pub fn github_token_configured() -> bool {
    github_token_from_env().is_some()
}

fn pr_client(client: &Client) -> GitHubPrClient {
    GitHubPrClient::with_env_token(client.clone(), "review-bee/0.1")
}

pub async fn fetch_review_context(
    client: &Client,
    repo: &str,
    pr_number: i64,
) -> Result<GitHubReviewContext> {
    let client = pr_client(client);
    let pr = client.fetch_pull_request(repo, pr_number).await?;
    let reviews = client.fetch_pull_request_reviews(repo, pr_number).await?;
    let threads = client
        .fetch_pull_request_review_threads(repo, pr_number)
        .await?;

    Ok(GitHubReviewContext {
        pr,
        reviews,
        threads,
    })
}
