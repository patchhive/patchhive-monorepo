#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/export-crate.sh <crate-name> [remote-name] [target-branch]

Examples:
  ./scripts/export-crate.sh patchhive-product-core
  ./scripts/export-crate.sh patchhive-product-core product-core main

What it does:
  1. Creates a subtree-export branch from crates/<crate-name>
  2. Optionally pushes that branch to a remote/branch you specify

Notes:
  - The monorepo remains the source of truth.
  - Standalone crate repositories are mirrors for visibility, package-focused
    issues, and reuse from exported product repositories.
  - If the default export branch already exists, a timestamped branch name is used
    instead of overwriting anything.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

CRATE_NAME="${1:-}"
REMOTE_NAME="${2:-}"
TARGET_BRANCH="${3:-main}"

if [[ -z "$CRATE_NAME" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

CRATE_PREFIX="crates/${CRATE_NAME}"
if [[ ! -d "$CRATE_PREFIX" ]]; then
  echo "PatchHive crate not found: ${CRATE_PREFIX}" >&2
  exit 1
fi

SANITIZED_NAME="${CRATE_NAME//\//-}"
EXPORT_BRANCH="export/crate-${SANITIZED_NAME}"

if git show-ref --verify --quiet "refs/heads/${EXPORT_BRANCH}"; then
  EXPORT_BRANCH="${EXPORT_BRANCH}-$(date +%Y%m%d-%H%M%S)"
fi

echo "Creating export branch ${EXPORT_BRANCH} from ${CRATE_PREFIX}..."
git subtree split --prefix="$CRATE_PREFIX" --branch "$EXPORT_BRANCH"

echo
echo "Created ${EXPORT_BRANCH}"

if [[ -n "$REMOTE_NAME" ]]; then
  echo "Pushing ${EXPORT_BRANCH} to ${REMOTE_NAME}:${TARGET_BRANCH}..."
  git push "$REMOTE_NAME" "${EXPORT_BRANCH}:${TARGET_BRANCH}"
  echo "Push complete."
fi

echo
echo "Next steps:"
echo "  1. Create or confirm a standalone repo for ${CRATE_NAME}."
echo "  2. Keep canonical crate development in the monorepo."
echo "  3. Re-export or mirror-sync the crate repo when you want its GitHub mirror updated."
