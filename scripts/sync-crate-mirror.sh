#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/sync-crate-mirror.sh <crate-name> <remote-name> [target-branch] [--reset-history]

Examples:
  ./scripts/sync-crate-mirror.sh patchhive-product-core product-core main
  ./scripts/sync-crate-mirror.sh patchhive-product-core product-core main --reset-history

What it does:
  1. Builds a crate-only working tree from crates/<crate-name>
  2. Commits it as a single mirror-sync commit
  3. Pushes it into the standalone crate repository

Notes:
  - Use export-crate.sh for the first crate repo export.
  - Use this script for ongoing crate mirror updates when you want clean,
    crate-specific commit history in the standalone repo.
  - --reset-history force-pushes a fresh root commit into the standalone repo.
EOF
}

RESET_HISTORY=false
POSITIONAL=()

for arg in "$@"; do
  case "$arg" in
    -h|--help)
      usage
      exit 0
      ;;
    --reset-history)
      RESET_HISTORY=true
      ;;
    *)
      POSITIONAL+=("$arg")
      ;;
  esac
done

set -- "${POSITIONAL[@]}"

CRATE_NAME="${1:-}"
REMOTE_NAME="${2:-}"
TARGET_BRANCH="${3:-main}"

if [[ -z "$CRATE_NAME" || -z "$REMOTE_NAME" ]]; then
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

if ! git remote get-url "$REMOTE_NAME" >/dev/null 2>&1; then
  echo "Git remote not found: ${REMOTE_NAME}" >&2
  exit 1
fi

REMOTE_REF="refs/remotes/${REMOTE_NAME}/${TARGET_BRANCH}"
BASE_REF=""

if git show-ref --verify --quiet "$REMOTE_REF"; then
  BASE_REF="${REMOTE_NAME}/${TARGET_BRANCH}"
elif [[ "$RESET_HISTORY" == false ]]; then
  echo "Remote-tracking ref ${REMOTE_NAME}/${TARGET_BRANCH} not found." >&2
  echo "Run ./scripts/export-crate.sh ${CRATE_NAME} ${REMOTE_NAME} ${TARGET_BRANCH} first," >&2
  echo "or rerun this command with --reset-history." >&2
  exit 1
else
  BASE_REF="HEAD"
fi

SANITIZED_NAME="${CRATE_NAME//\//-}"
WORKTREE_DIR="$(mktemp -d "/tmp/patchhive-crate-sync-${SANITIZED_NAME}.XXXXXX")"
SYNC_BRANCH="mirror-sync-${SANITIZED_NAME}"
SOURCE_SHA="$(git rev-parse --short HEAD)"

cleanup() {
  git worktree remove --force "$WORKTREE_DIR" >/dev/null 2>&1 || true
  if [[ "$RESET_HISTORY" == true ]] && git show-ref --verify --quiet "refs/heads/${SYNC_BRANCH}"; then
    git branch -D "$SYNC_BRANCH" >/dev/null 2>&1 || true
  fi
  rm -rf "$WORKTREE_DIR"
}

trap cleanup EXIT

echo "Preparing temporary worktree from ${BASE_REF}..."
git worktree add --detach "$WORKTREE_DIR" "$BASE_REF" >/dev/null

pushd "$WORKTREE_DIR" >/dev/null

if [[ "$RESET_HISTORY" == true ]]; then
  echo "Resetting standalone history for ${CRATE_NAME}..."
  git checkout --orphan "$SYNC_BRANCH" >/dev/null 2>&1
fi

git rm -r --quiet . >/dev/null 2>&1 || true

popd >/dev/null

git archive --format=tar "HEAD:${CRATE_PREFIX}" | tar -xf - -C "$WORKTREE_DIR"

pushd "$WORKTREE_DIR" >/dev/null

git add .

if [[ "$RESET_HISTORY" == false ]] && git diff --cached --quiet; then
  echo "No crate changes to sync for ${CRATE_NAME}."
  exit 0
fi

COMMIT_MESSAGE="Sync ${CRATE_NAME} mirror from monorepo (${SOURCE_SHA})"
git commit -m "$COMMIT_MESSAGE" >/dev/null

if [[ "$RESET_HISTORY" == true ]]; then
  echo "Force-pushing clean mirror history to ${REMOTE_NAME}:${TARGET_BRANCH}..."
  git push --force "$REMOTE_NAME" "HEAD:${TARGET_BRANCH}"
else
  echo "Pushing squash-sync commit to ${REMOTE_NAME}:${TARGET_BRANCH}..."
  git push "$REMOTE_NAME" "HEAD:${TARGET_BRANCH}"
fi

popd >/dev/null

echo "Standalone crate mirror updated."
