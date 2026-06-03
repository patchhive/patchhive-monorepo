#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/export-service.sh <service-name> [remote-name-or-url] [target-branch]

Examples:
  ./scripts/export-service.sh patchhive-backend
  ./scripts/export-service.sh patchhive-backend https://github.com/patchhive/patchhive-unified-backend.git main

What it does:
  1. Creates a subtree-export branch from services/<service-name>
  2. Optionally pushes that branch to a remote/branch you specify

Notes:
  - The monorepo remains the source of truth.
  - Standalone service repositories are mirrors for visibility, releases,
    issues, and Docker image build context.
  - Set PATCHHIVE_EXPORT_FORCE_WITH_LEASE=1 when updating a standalone mirror
    that may already have generated-only mirror commits on its target branch.
  - If the default export branch already exists, a timestamped branch name is used
    instead of overwriting anything.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

SERVICE_NAME="${1:-}"
REMOTE_NAME="${2:-}"
TARGET_BRANCH="${3:-main}"

if [[ -z "$SERVICE_NAME" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

SERVICE_PREFIX="services/${SERVICE_NAME}"
if [[ ! -d "$SERVICE_PREFIX" ]]; then
  echo "PatchHive service not found: ${SERVICE_PREFIX}" >&2
  exit 1
fi

SANITIZED_NAME="${SERVICE_NAME//\//-}"
EXPORT_BRANCH="export/service-${SANITIZED_NAME}"

if git show-ref --verify --quiet "refs/heads/${EXPORT_BRANCH}"; then
  EXPORT_BRANCH="${EXPORT_BRANCH}-$(date +%Y%m%d-%H%M%S)"
fi

echo "Creating export branch ${EXPORT_BRANCH} from ${SERVICE_PREFIX}..."
git subtree split --prefix="$SERVICE_PREFIX" --branch "$EXPORT_BRANCH"

echo
echo "Created ${EXPORT_BRANCH}"

if [[ -n "$REMOTE_NAME" ]]; then
  echo "Pushing ${EXPORT_BRANCH} to ${REMOTE_NAME}:${TARGET_BRANCH}..."
  if [[ "${PATCHHIVE_EXPORT_FORCE_WITH_LEASE:-0}" == "1" ]]; then
    REMOTE_SHA="$(git ls-remote "$REMOTE_NAME" "refs/heads/${TARGET_BRANCH}" | awk '{print $1}')"
    if [[ -n "$REMOTE_SHA" ]]; then
      git push --force-with-lease="${TARGET_BRANCH}:${REMOTE_SHA}" "$REMOTE_NAME" "${EXPORT_BRANCH}:${TARGET_BRANCH}"
    else
      git push "$REMOTE_NAME" "${EXPORT_BRANCH}:${TARGET_BRANCH}"
    fi
  else
    git push "$REMOTE_NAME" "${EXPORT_BRANCH}:${TARGET_BRANCH}"
  fi
  echo "Push complete."
fi

echo
echo "Next steps:"
echo "  1. Keep canonical service development in the monorepo."
echo "  2. Use this script when you want the standalone service mirror updated."
echo "  3. Build Docker images from the exported service repository or the monorepo service path."
