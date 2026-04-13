# patchhive-github-data

Shared Rust GitHub data client for PatchHive products.

This crate holds the repeated GitHub read-paths that started showing up across
SignalHive, RepoMemory, and FlakeSting.

## Current Scope

- standard PatchHive GitHub token/env resolution
- repository fetch and repository search
- issue and merged-PR history reads
- pull-request review/comment/file reads for historical ingestion
- code search count reads
- GitHub Actions workflow run and workflow job reads

## Intent

`patchhive-github-data` should stay focused on GitHub data access and typed
response shapes.

It should not absorb:

- PR webhook verification or PR comment/check publishing
- product scoring heuristics
- product policy logic
- product-specific route behavior

That keeps the boundary clean alongside `patchhive-github-pr`, which owns the
PR lifecycle plumbing instead.
