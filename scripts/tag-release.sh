#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/tag-release.sh <repo-reaper|ui|ai-local> <version> [--dry-run] [--monorepo-only] [--standalone-only]

Examples:
  ./scripts/tag-release.sh repo-reaper v0.1.0
  ./scripts/tag-release.sh ui 0.1.1
  ./scripts/tag-release.sh ai-local v0.1.0 --dry-run

What it does:
  - creates a namespaced annotated tag in the monorepo, such as repo-reaper/v0.1.0
  - pushes that namespaced tag to origin
  - creates and pushes a standalone vX.Y.Z tag to the matching mirror repo

Notes:
  - Version may be provided with or without a leading v.
  - Use --monorepo-only or --standalone-only if you only want one side.
  - This script assumes the standalone mirror is already synced to the desired commit.
EOF
}

TARGET=""
VERSION=""
DRY_RUN=false
MONOREPO_ONLY=false
STANDALONE_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --dry-run)
      DRY_RUN=true
      ;;
    --monorepo-only)
      MONOREPO_ONLY=true
      ;;
    --standalone-only)
      STANDALONE_ONLY=true
      ;;
    *)
      if [[ -z "$TARGET" ]]; then
        TARGET="$1"
      elif [[ -z "$VERSION" ]]; then
        VERSION="$1"
      else
        echo "Unexpected argument: $1" >&2
        usage
        exit 1
      fi
      ;;
  esac
  shift
done

if [[ -z "$TARGET" || -z "$VERSION" ]]; then
  usage
  exit 1
fi

if [[ "$MONOREPO_ONLY" == true && "$STANDALONE_ONLY" == true ]]; then
  echo "Choose either --monorepo-only or --standalone-only, not both." >&2
  exit 1
fi

case "$TARGET" in
  repo-reaper)
    MONOREPO_TAG_PREFIX="repo-reaper"
    STANDALONE_REMOTE="repo-reaper"
    STANDALONE_REF="repo-reaper/main"
    ;;
  ui)
    MONOREPO_TAG_PREFIX="ui"
    STANDALONE_REMOTE="patchhive-ui"
    STANDALONE_REF="patchhive-ui/main"
    ;;
  ai-local)
    MONOREPO_TAG_PREFIX="ai-local"
    STANDALONE_REMOTE="patchhive-ai-local"
    STANDALONE_REF="patchhive-ai-local/main"
    ;;
  *)
    echo "Unknown target: $TARGET" >&2
    usage
    exit 1
    ;;
esac

if [[ "$VERSION" != v* ]]; then
  VERSION="v${VERSION}"
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

MONOREPO_TAG="${MONOREPO_TAG_PREFIX}/${VERSION}"
TEMP_TAG="tmp-${MONOREPO_TAG_PREFIX//\//-}-${VERSION}-$$"

cleanup() {
  if git show-ref --verify --quiet "refs/tags/${TEMP_TAG}"; then
    git tag -d "$TEMP_TAG" >/dev/null 2>&1 || true
  fi
}

trap cleanup EXIT

if [[ "$STANDALONE_ONLY" == false ]]; then
  if git show-ref --verify --quiet "refs/tags/${MONOREPO_TAG}"; then
    echo "Monorepo tag already exists: ${MONOREPO_TAG}" >&2
    exit 1
  fi
fi

if [[ "$MONOREPO_ONLY" == false ]]; then
  if ! git show-ref --verify --quiet "refs/remotes/${STANDALONE_REF}"; then
    echo "Standalone mirror ref not found locally: ${STANDALONE_REF}" >&2
    echo "Sync the mirror first, or fetch that remote, before tagging." >&2
    exit 1
  fi
fi

echo "Release target: ${TARGET}"
echo "Version: ${VERSION}"

if [[ "$STANDALONE_ONLY" == false ]]; then
  echo "Monorepo tag: ${MONOREPO_TAG} -> HEAD"
fi

if [[ "$MONOREPO_ONLY" == false ]]; then
  echo "Standalone tag: ${VERSION} -> ${STANDALONE_REF}"
fi

if [[ "$DRY_RUN" == true ]]; then
  exit 0
fi

if [[ "$STANDALONE_ONLY" == false ]]; then
  git tag -a "$MONOREPO_TAG" -m "Release ${MONOREPO_TAG}"
  git push origin "refs/tags/${MONOREPO_TAG}"
fi

if [[ "$MONOREPO_ONLY" == false ]]; then
  git tag -a "$TEMP_TAG" "$STANDALONE_REF" -m "Release ${TARGET} ${VERSION}"
  git push "$STANDALONE_REMOTE" "refs/tags/${TEMP_TAG}:refs/tags/${VERSION}"
  git tag -d "$TEMP_TAG" >/dev/null
fi

echo "Release tags pushed."
