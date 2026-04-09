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
  2. Regenerates backend/Cargo.lock there without the monorepo's local crate patch
  3. Copies the standalone-safe lockfile back into the product directory

Use this before the first export, or whenever a product backend's shared git crate
dependencies change and the standalone repo needs a fresh lockfile.
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

cp -R "$PRODUCT_DIR" "$TMP_DIR/product"
rm -f "$TMP_DIR/product/backend/Cargo.lock"
(
  cd "$TMP_DIR/product/backend"
  cargo generate-lockfile
)
cp "$TMP_DIR/product/backend/Cargo.lock" "$BACKEND_DIR/Cargo.lock"

echo "Refreshed standalone Cargo.lock for $PRODUCT_NAME"
