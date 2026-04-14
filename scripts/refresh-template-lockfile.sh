#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/refresh-template-lockfile.sh <template-name>

Example:
  ./scripts/refresh-template-lockfile.sh product-starter

What it does:
  1. Copies templates/<template-name>/scaffold to a temporary directory outside the monorepo
  2. Regenerates scaffold/backend/Cargo.lock there without the monorepo's local crate patch
  3. Copies the standalone-safe lockfile back into the template scaffold

Use this whenever a template scaffold backend's shared git crate dependencies change
and the standalone template repo needs a fresh lockfile for `cargo check --locked`.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

TEMPLATE_NAME="${1:-}"
if [[ -z "$TEMPLATE_NAME" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
TEMPLATE_DIR="$ROOT_DIR/templates/$TEMPLATE_NAME"
SCAFFOLD_DIR="$TEMPLATE_DIR/scaffold"
BACKEND_DIR="$SCAFFOLD_DIR/backend"

if [[ ! -d "$BACKEND_DIR" ]]; then
  echo "Template scaffold backend not found: $BACKEND_DIR" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d /tmp/patchhive-template-lockfile-XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT

cp -R "$SCAFFOLD_DIR" "$TMP_DIR/scaffold"
rm -f "$TMP_DIR/scaffold/backend/Cargo.lock"
(
  cd "$TMP_DIR/scaffold/backend"
  cargo generate-lockfile
)
cp "$TMP_DIR/scaffold/backend/Cargo.lock" "$BACKEND_DIR/Cargo.lock"

echo "Refreshed standalone Cargo.lock for template ${TEMPLATE_NAME}"
