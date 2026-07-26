#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/refresh-crate-lockfile.sh <crate-name>

Example:
  ./scripts/refresh-crate-lockfile.sh patchhive-github-security

What it does:
  1. Copies crates/<crate-name> to a temporary directory outside the monorepo
  2. Rewrites shared PatchHive path dependencies to pinned standalone mirrors
  3. Regenerates Cargo.lock there without monorepo-only paths
  4. Copies the standalone-safe lockfile back into the crate directory

Use this whenever a shared crate's git dependencies change and its standalone
repository needs a fresh lockfile for `cargo check --locked`.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

CRATE_NAME="${1:-}"
if [[ -z "$CRATE_NAME" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
CRATE_DIR="$ROOT_DIR/crates/$CRATE_NAME"

if [[ ! -f "$CRATE_DIR/Cargo.toml" ]]; then
  echo "Crate not found: $CRATE_DIR" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d /tmp/patchhive-crate-lockfile-XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT

cp -R "$CRATE_DIR" "$TMP_DIR/crate"
"$ROOT_DIR/scripts/prepare-standalone-cargo-manifest.sh" --git-mirrors "$TMP_DIR/crate/Cargo.toml"
rm -f "$TMP_DIR/crate/Cargo.lock"
(
  cd "$TMP_DIR/crate"
  cargo generate-lockfile
)
cp "$TMP_DIR/crate/Cargo.lock" "$CRATE_DIR/Cargo.lock"

echo "Refreshed standalone Cargo.lock for $CRATE_NAME"
