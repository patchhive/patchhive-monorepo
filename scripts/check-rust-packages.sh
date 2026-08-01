#!/usr/bin/env bash
set -euo pipefail

# PatchHive is intentionally not a single Cargo workspace. Check every
# standalone Rust package in the shared manifest inventory so CI catches drift
# across products, services, crates, and local gateway code.

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

while IFS= read -r manifest; do
  [[ -n "${manifest}" ]] || continue
  echo "::group::cargo fmt ${manifest}"
  cargo fmt --manifest-path "${manifest}" -- --check
  echo "::endgroup::"

  echo "::group::cargo clippy ${manifest}"
  cargo clippy --locked --all-targets --manifest-path "${manifest}" -- -D warnings
  echo "::endgroup::"

  echo "::group::cargo test ${manifest}"
  cargo test --locked --all-targets --manifest-path "${manifest}"
  echo "::endgroup::"
done < "${script_dir}/rust-manifests.txt"
