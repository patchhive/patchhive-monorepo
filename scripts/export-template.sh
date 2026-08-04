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
  - Exports require a clean worktree and use committed HEAD exactly.
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

# shellcheck source=scripts/suite-common.sh
source "$ROOT_DIR/scripts/suite-common.sh"

patchhive_require_inventory_item "template" "$TEMPLATE_NAME" "${PATCHHIVE_TEMPLATES[@]}"
patchhive_require_branch_name "$TARGET_BRANCH"
patchhive_require_clean_worktree
if [[ -n "$REMOTE_NAME" ]]; then
  patchhive_require_remote_operand "$REMOTE_NAME"
fi

TEMPLATE_PREFIX="templates/${TEMPLATE_NAME}"
if [[ ! -d "$TEMPLATE_PREFIX" ]]; then
  echo "PatchHive template not found: ${TEMPLATE_PREFIX}" >&2
  exit 1
fi

TMP_PATHS=()
LOCKFILE_PATH="${TEMPLATE_PREFIX}/scaffold/backend/Cargo.lock"
ORIGINAL_LOCKFILE="$(mktemp "/tmp/patchhive-${TEMPLATE_NAME}-Cargo.lock-original-XXXXXX")"
STANDALONE_LOCKFILE="$(mktemp "/tmp/patchhive-${TEMPLATE_NAME}-Cargo.lock-standalone-XXXXXX")"
TMP_PATHS+=("$ORIGINAL_LOCKFILE" "$STANDALONE_LOCKFILE")
LOCKFILE_EXISTED=0
RESTORE_LOCKFILE=true
EXPORT_WORKTREE=""

cleanup() {
  if [[ -n "$EXPORT_WORKTREE" ]]; then
    git worktree remove --force "$EXPORT_WORKTREE" >/dev/null 2>&1 || true
  fi
  if [[ "$RESTORE_LOCKFILE" == true ]]; then
    if [[ "$LOCKFILE_EXISTED" -eq 1 ]]; then
      cp "$ORIGINAL_LOCKFILE" "$LOCKFILE_PATH"
    else
      rm -f "$LOCKFILE_PATH"
    fi
  fi
  for path in "${TMP_PATHS[@]}"; do
    rm -rf "$path"
  done
}
trap cleanup EXIT

if [[ -f "$LOCKFILE_PATH" ]]; then
  cp "$LOCKFILE_PATH" "$ORIGINAL_LOCKFILE"
  LOCKFILE_EXISTED=1
fi

echo "Refreshing standalone Cargo.lock for template ${TEMPLATE_NAME} before export..."
"$ROOT_DIR/scripts/refresh-template-lockfile.sh" "$TEMPLATE_NAME"
cp "$LOCKFILE_PATH" "$STANDALONE_LOCKFILE"
if [[ "$LOCKFILE_EXISTED" -eq 1 ]]; then
  cp "$ORIGINAL_LOCKFILE" "$LOCKFILE_PATH"
else
  rm -f "$LOCKFILE_PATH"
fi
RESTORE_LOCKFILE=false

SANITIZED_NAME="${TEMPLATE_NAME//\//-}"
EXPORT_BRANCH="export-template/${SANITIZED_NAME}"

if git show-ref --verify --quiet "refs/heads/${EXPORT_BRANCH}"; then
  EXPORT_BRANCH="${EXPORT_BRANCH}-$(date +%Y%m%d-%H%M%S)"
fi

echo "Creating export branch ${EXPORT_BRANCH} from ${TEMPLATE_PREFIX}..."
git subtree split --prefix="$TEMPLATE_PREFIX" --branch "$EXPORT_BRANCH"

EXPORT_WORKTREE="$(mktemp -d "/tmp/patchhive-${SANITIZED_NAME}-export-XXXXXX")"
TMP_PATHS+=("$EXPORT_WORKTREE")
git worktree add "$EXPORT_WORKTREE" "$EXPORT_BRANCH" >/dev/null
cp "$STANDALONE_LOCKFILE" "$EXPORT_WORKTREE/scaffold/backend/Cargo.lock"
if ! git -C "$EXPORT_WORKTREE" diff --quiet; then
  git -C "$EXPORT_WORKTREE" add -A
  git -C "$EXPORT_WORKTREE" commit -m "chore: refresh standalone lockfile"
fi
git worktree remove "$EXPORT_WORKTREE" >/dev/null
EXPORT_WORKTREE=""

echo
echo "Created ${EXPORT_BRANCH}"

if [[ -n "$REMOTE_NAME" ]]; then
  echo "Pushing ${EXPORT_BRANCH} to ${REMOTE_NAME}:${TARGET_BRANCH}..."
  git push -- "$REMOTE_NAME" "${EXPORT_BRANCH}:${TARGET_BRANCH}"
  echo "Push complete."
fi

echo
echo "Next steps:"
echo "  1. Keep developing the template in the monorepo."
echo "  2. Re-export the template when starter improvements land."
echo "  3. Treat the standalone template repo as a mirror, not the source of truth."
