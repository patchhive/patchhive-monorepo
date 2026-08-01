#!/usr/bin/env bash
set -euo pipefail

# CodeQL manual build entrypoint. PatchHive intentionally has no root Cargo
# workspace, so every standalone package must be compiled explicitly for the
# extractor to resolve types, calls, dependencies, and generated code.

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

while IFS= read -r manifest; do
  [[ -n "${manifest}" ]] || continue
  echo "::group::cargo build ${manifest}"
  cargo build --locked --all-targets --manifest-path "${manifest}"
  echo "::endgroup::"
done < "${script_dir}/rust-manifests.txt"
