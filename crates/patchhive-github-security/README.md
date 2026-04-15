# patchhive-github-security

`patchhive-github-security` is the shared Rust security data client for PatchHive products.

It owns the GitHub security and advisory reads that should stay consistent across products such as VulnTriage and DepTriage.

## Current Scope

- PatchHive-standard GitHub token and env resolution through `patchhive-github-data`
- Dependabot alert reads
- code scanning alert reads
- typed advisory metadata, references, CWEs, and EPSS fields

## Design Boundary

This crate is for security data access, not product ranking.

Products should keep their own scoring, prioritization, and explanation logic on top of the typed security data this crate provides.
