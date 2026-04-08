#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/export-product.sh <product-name> [remote-name] [target-branch]

Examples:
  ./scripts/export-product.sh repo-reaper
  ./scripts/export-product.sh repo-reaper repo-reaper main

What it does:
  1. Creates a subtree-export branch from products/<product-name>
  2. Optionally pushes that branch to a remote/branch you specify

Notes:
  - The monorepo remains the source of truth.
  - Shared packages are not copied into the export. Standalone product repos
    should depend on published @patchhive/* packages or shared service contracts.
  - If the default export branch already exists, a timestamped branch name is used
    instead of overwriting anything.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

PRODUCT_NAME="${1:-}"
REMOTE_NAME="${2:-}"
TARGET_BRANCH="${3:-main}"

if [[ -z "$PRODUCT_NAME" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

PRODUCT_PREFIX="products/${PRODUCT_NAME}"
if [[ ! -d "$PRODUCT_PREFIX" ]]; then
  echo "PatchHive product not found: ${PRODUCT_PREFIX}" >&2
  exit 1
fi

SANITIZED_NAME="${PRODUCT_NAME//\//-}"
EXPORT_BRANCH="export/${SANITIZED_NAME}"

if git show-ref --verify --quiet "refs/heads/${EXPORT_BRANCH}"; then
  EXPORT_BRANCH="${EXPORT_BRANCH}-$(date +%Y%m%d-%H%M%S)"
fi

echo "Creating export branch ${EXPORT_BRANCH} from ${PRODUCT_PREFIX}..."
git subtree split --prefix="$PRODUCT_PREFIX" --branch "$EXPORT_BRANCH"

echo
echo "Created ${EXPORT_BRANCH}"

if [[ -n "$REMOTE_NAME" ]]; then
  echo "Pushing ${EXPORT_BRANCH} to ${REMOTE_NAME}:${TARGET_BRANCH}..."
  git push "$REMOTE_NAME" "${EXPORT_BRANCH}:${TARGET_BRANCH}"
  echo "Push complete."
fi

echo
echo "Next steps:"
echo "  1. Create or confirm a standalone repo for ${PRODUCT_NAME}."
echo "  2. Publish shared packages such as @patchhivehq/ui before wiring the exported frontend."
echo "  3. Keep developing in the monorepo, then re-export when needed."
