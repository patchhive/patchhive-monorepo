#!/usr/bin/env bash
set -euo pipefail

# CodeQL's Rust extractor gets better call-target resolution when it observes
# real Cargo builds. PatchHive is intentionally not a single Cargo workspace, so
# this script checks each standalone Rust package explicitly.

manifests=(
  "crates/patchhive-product-core/Cargo.toml"
  "crates/patchhive-github-data/Cargo.toml"
  "crates/patchhive-github-pr/Cargo.toml"
  "crates/patchhive-github-security/Cargo.toml"
  "services/patchhive-backend/Cargo.toml"
  "services/patchhive-launcher/Cargo.toml"
  "services/patchhive-registry/Cargo.toml"
  "packages/ai-local/rust-gateway/Cargo.toml"
  "products/repo-reaper/backend/Cargo.toml"
  "products/signal-hive/backend/Cargo.toml"
  "products/review-bee/backend/Cargo.toml"
  "products/trust-gate/backend/Cargo.toml"
  "products/repo-memory/backend/Cargo.toml"
  "products/merge-keeper/backend/Cargo.toml"
  "products/flake-sting/backend/Cargo.toml"
  "products/dep-triage/backend/Cargo.toml"
  "products/vuln-triage/backend/Cargo.toml"
  "products/refactor-scout/backend/Cargo.toml"
  "products/release-sentry/backend/Cargo.toml"
  "products/hive-core/backend/Cargo.toml"
)

for manifest in "${manifests[@]}"; do
  echo "::group::cargo check ${manifest}"
  cargo check --locked --all-targets --manifest-path "${manifest}"
  echo "::endgroup::"
done
