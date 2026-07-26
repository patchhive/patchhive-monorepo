#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/refresh-product-lockfile.sh <product-slug>

Example:
  ./scripts/refresh-product-lockfile.sh trust-gate

What it does:
  1. Copies products/<product-slug> to a temporary directory outside the monorepo
  2. Copies the current shared-crate snapshot used by standalone exports
  3. Rewrites shared PatchHive dependencies to that snapshot
  4. Regenerates backend/Cargo.lock there without monorepo-only paths
  5. Copies the standalone-safe lockfile back into the product directory

Use this before the first export, or whenever a product backend or shared crate
dependency changes and the standalone repo needs a fresh lockfile.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

PRODUCT_NAME="${1:-}"
if [[ -z "$PRODUCT_NAME" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
PRODUCT_DIR="$ROOT_DIR/products/$PRODUCT_NAME"
BACKEND_DIR="$PRODUCT_DIR/backend"

if [[ ! -d "$BACKEND_DIR" ]]; then
  echo "Product backend not found: $BACKEND_DIR" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d /tmp/patchhive-lockfile-XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$TMP_DIR/product"
rsync -a --exclude target/ --exclude node_modules/ "$PRODUCT_DIR/" "$TMP_DIR/product/"
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

echo "Refreshed standalone Cargo.lock for $PRODUCT_NAME"
