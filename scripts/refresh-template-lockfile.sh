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
  2. Copies the current shared-crate snapshot used by product exports
  3. Rewrites PatchHive dependencies to the standalone snapshot layout
  4. Regenerates backend/Cargo.lock there without monorepo-only paths
  5. Copies the standalone-safe lockfile back into the template scaffold

Use this whenever a template scaffold backend's shared crate dependencies change
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
# shellcheck source=scripts/suite-common.sh
source "$ROOT_DIR/scripts/suite-common.sh"
patchhive_require_inventory_item "template" "$TEMPLATE_NAME" "${PATCHHIVE_TEMPLATES[@]}"
TEMPLATE_DIR="$ROOT_DIR/templates/$TEMPLATE_NAME"
SCAFFOLD_DIR="$TEMPLATE_DIR/scaffold"
BACKEND_DIR="$SCAFFOLD_DIR/backend"

if [[ ! -d "$BACKEND_DIR" ]]; then
  echo "Template scaffold backend not found: $BACKEND_DIR" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d /tmp/patchhive-template-lockfile-XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$TMP_DIR/product"
rsync -a --exclude target/ --exclude node_modules/ "$SCAFFOLD_DIR/" "$TMP_DIR/product/"
mkdir -p "$TMP_DIR/product/shared-crates"
for crate in \
  patchhive-product-core \
  patchhive-github-pr \
  patchhive-github-data \
  patchhive-github-security; do
  mkdir -p "$TMP_DIR/product/shared-crates/$crate"
  rsync -a --exclude target/ "$ROOT_DIR/crates/$crate/" "$TMP_DIR/product/shared-crates/$crate/"
done
"$ROOT_DIR/scripts/prepare-standalone-cargo-manifest.sh" "$TMP_DIR/product/backend/Cargo.toml"
rm -f "$TMP_DIR/product/backend/Cargo.lock"
(
  cd "$TMP_DIR/product/backend"
  cargo generate-lockfile
)
cp "$TMP_DIR/product/backend/Cargo.lock" "$BACKEND_DIR/Cargo.lock"

echo "Refreshed standalone Cargo.lock for template ${TEMPLATE_NAME}"
