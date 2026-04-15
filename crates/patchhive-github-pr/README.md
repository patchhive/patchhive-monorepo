# patchhive-github-pr

`patchhive-github-pr` is the shared Rust pull request plumbing crate for PatchHive.

It exists for the GitHub mechanics that PR-driven products should not each reimplement for themselves: webhook verification, pull request fetch paths, check publishing, status publishing, and maintained pull request comments.

## Current Scope

- PatchHive-standard GitHub token and env resolution
- signed GitHub webhook verification
- pull request metadata, diff, review, thread, and commit-health reads
- GitHub check run publishing
- commit status publishing
- maintained pull request comment upsert

## Design Boundary

This crate is intentionally plumbing, not product logic.

Products such as TrustGate, ReviewBee, and MergeKeeper should keep their own scoring, report language, and policy decisions local while using this crate for GitHub transport and lifecycle operations.
