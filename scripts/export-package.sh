#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/export-package.sh <package-name> [remote-name] [target-branch]

Examples:
  ./scripts/export-package.sh ui
  ./scripts/export-package.sh ui patchhive-ui main

What it does:
  1. Creates a subtree-export branch from packages/<package-name>
  2. Optionally pushes that branch to a remote/branch you specify

Notes:
  - The monorepo remains the source of truth.
  - Standalone package repositories are mirrors for visibility, package-focused
    issues, and external consumption.
  - If the default export branch already exists, a timestamped branch name is used
    instead of overwriting anything.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

PACKAGE_NAME="${1:-}"
REMOTE_NAME="${2:-}"
TARGET_BRANCH="${3:-main}"

if [[ -z "$PACKAGE_NAME" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

PACKAGE_PREFIX="packages/${PACKAGE_NAME}"
if [[ ! -d "$PACKAGE_PREFIX" ]]; then
  echo "PatchHive package not found: ${PACKAGE_PREFIX}" >&2
  exit 1
fi

SANITIZED_NAME="${PACKAGE_NAME//\//-}"
EXPORT_BRANCH="export/package-${SANITIZED_NAME}"

if git show-ref --verify --quiet "refs/heads/${EXPORT_BRANCH}"; then
  EXPORT_BRANCH="${EXPORT_BRANCH}-$(date +%Y%m%d-%H%M%S)"
fi

echo "Creating export branch ${EXPORT_BRANCH} from ${PACKAGE_PREFIX}..."
git subtree split --prefix="$PACKAGE_PREFIX" --branch "$EXPORT_BRANCH"

echo
echo "Created ${EXPORT_BRANCH}"

if [[ -n "$REMOTE_NAME" ]]; then
  echo "Pushing ${EXPORT_BRANCH} to ${REMOTE_NAME}:${TARGET_BRANCH}..."
  git push "$REMOTE_NAME" "${EXPORT_BRANCH}:${TARGET_BRANCH}"
  echo "Push complete."
fi

echo
echo "Next steps:"
echo "  1. Create or confirm a standalone repo for ${PACKAGE_NAME}."
echo "  2. Keep publishing package releases from the monorepo source of truth."
echo "  3. Re-export the package repo when you want its GitHub mirror updated."
