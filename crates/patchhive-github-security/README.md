# patchhive-github-security

Shared Rust GitHub security data client for PatchHive products.

This crate holds the repeated GitHub security read paths that started showing up
across DepTriage and VulnTriage.

## Current Scope

- standard PatchHive GitHub token/env resolution via `patchhive-github-data`
- Dependabot alert reads
- code scanning alert reads
- advisory metadata, CWEs, references, and EPSS fields

## Intent

`patchhive-github-security` should stay focused on typed GitHub security reads.

It should not absorb:

- PR webhook verification or PR comment/check publishing
- generic repository or issue history reads
- product-specific scoring heuristics
- product-specific route behavior or policy logic

That keeps the boundary clean alongside `patchhive-github-data`, which owns the
broader GitHub read layer, and `patchhive-github-pr`, which owns PR lifecycle
plumbing.
