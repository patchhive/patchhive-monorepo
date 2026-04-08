#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/export-template.sh <template-name> [remote-name] [target-branch]

Examples:
  ./scripts/export-template.sh product-starter
  ./scripts/export-template.sh product-starter product-starter main

What it does:
  1. Creates a subtree-export branch from templates/<template-name>
  2. Optionally pushes that branch to a remote/branch you specify

Notes:
  - The monorepo remains the source of truth.
  - Standalone template repositories should be treated as mirrors of the template directory.
  - If the default export branch already exists, a timestamped branch name is used
    instead of overwriting anything.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

TEMPLATE_NAME="${1:-}"
REMOTE_NAME="${2:-}"
TARGET_BRANCH="${3:-main}"

if [[ -z "$TEMPLATE_NAME" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

TEMPLATE_PREFIX="templates/${TEMPLATE_NAME}"
if [[ ! -d "$TEMPLATE_PREFIX" ]]; then
  echo "PatchHive template not found: ${TEMPLATE_PREFIX}" >&2
  exit 1
fi

SANITIZED_NAME="${TEMPLATE_NAME//\//-}"
EXPORT_BRANCH="export-template/${SANITIZED_NAME}"

if git show-ref --verify --quiet "refs/heads/${EXPORT_BRANCH}"; then
  EXPORT_BRANCH="${EXPORT_BRANCH}-$(date +%Y%m%d-%H%M%S)"
fi

echo "Creating export branch ${EXPORT_BRANCH} from ${TEMPLATE_PREFIX}..."
git subtree split --prefix="$TEMPLATE_PREFIX" --branch "$EXPORT_BRANCH"

echo
echo "Created ${EXPORT_BRANCH}"

if [[ -n "$REMOTE_NAME" ]]; then
  echo "Pushing ${EXPORT_BRANCH} to ${REMOTE_NAME}:${TARGET_BRANCH}..."
  git push "$REMOTE_NAME" "${EXPORT_BRANCH}:${TARGET_BRANCH}"
  echo "Push complete."
fi

echo
echo "Next steps:"
echo "  1. Keep developing the template in the monorepo."
echo "  2. Re-export the template when starter improvements land."
echo "  3. Treat the standalone template repo as a mirror, not the source of truth."
