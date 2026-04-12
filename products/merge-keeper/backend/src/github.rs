use anyhow::Result;
use patchhive_github_pr::{
    github_token_from_env, GitHubCommitHealth, GitHubPrClient, GitHubPullRequest,
    GitHubPullReview, GitHubPullReviewThread,
};
use reqwest::Client;

pub struct GitHubMergeContext {
    pub pr: GitHubPullRequest,
    pub reviews: Vec<GitHubPullReview>,
    pub threads: Vec<GitHubPullReviewThread>,
    pub commit_health: GitHubCommitHealth,
}

pub fn github_token_configured() -> bool {
    github_token_from_env().is_some()
}

fn pr_client(client: &Client) -> GitHubPrClient {
    GitHubPrClient::with_env_token(client.clone(), "merge-keeper/0.1")
}

pub async fn fetch_merge_context(
    client: &Client,
    repo: &str,
    pr_number: i64,
) -> Result<GitHubMergeContext> {
    let client = pr_client(client);
    let pr = client.fetch_pull_request(repo, pr_number).await?;
    let reviews = client.fetch_pull_request_reviews(repo, pr_number).await?;
    let threads = client
        .fetch_pull_request_review_threads(repo, pr_number)
        .await?;
    let commit_health = client.fetch_commit_health(repo, &pr.head_sha).await?;

    Ok(GitHubMergeContext {
        pr,
        reviews,
        threads,
        commit_health,
    })
}
