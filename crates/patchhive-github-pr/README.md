# patchhive-github-pr

Shared Rust GitHub pull-request plumbing for PatchHive products.

This crate focuses on the repeated mechanics around GitHub PR-driven products:

- resolving GitHub tokens from the standard PatchHive env vars
- verifying signed GitHub webhooks
- fetching PR metadata, merge/commit health, reviews, review threads, and unified diffs
- publishing GitHub check runs and commit statuses
- maintaining one managed PR comment instead of spamming review threads

It is intentionally plumbing, not product logic. Products like TrustGate should
format their own policy/report content and call this crate for transport and
GitHub API operations.
