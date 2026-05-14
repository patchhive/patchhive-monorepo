#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/smoke-frontend-package-deps.sh <product-name> [--ui-tarball <path>] [--product-shell-tarball <path>] [--keep]

Examples:
  ./scripts/smoke-frontend-package-deps.sh hive-core
  ./scripts/smoke-frontend-package-deps.sh hive-core --ui-tarball /tmp/patchhivehq-ui-0.1.3.tgz

What it does:
  Copies products/<product-name>/frontend into /tmp, installs dependencies without
  monorepo workspace links, and runs the product frontend build. This catches
  exported-product drift where a standalone repo uses published packages that
  differ from the monorepo workspace source.

Environment:
  PATCHHIVE_UI_TARBALL             Optional local @patchhivehq/ui tarball.
  PATCHHIVE_PRODUCT_SHELL_TARBALL  Optional local @patchhivehq/product-shell tarball.
  PATCHHIVE_NPM_CACHE_DIR          Optional npm cache directory. Defaults to /tmp/patchhive-npm-cache.
EOF
}

PRODUCT_NAME=""
UI_TARBALL="${PATCHHIVE_UI_TARBALL:-}"
PRODUCT_SHELL_TARBALL="${PATCHHIVE_PRODUCT_SHELL_TARBALL:-}"
KEEP_WORKTREE=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --ui-tarball)
      UI_TARBALL="${2:-}"
      shift 2
      ;;
    --product-shell-tarball)
      PRODUCT_SHELL_TARBALL="${2:-}"
      shift 2
      ;;
    --keep)
      KEEP_WORKTREE=true
      shift
      ;;
    -*)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
    *)
      if [[ -n "$PRODUCT_NAME" ]]; then
        echo "Unexpected extra argument: $1" >&2
        usage
        exit 1
      fi
      PRODUCT_NAME="$1"
      shift
      ;;
  esac
done

if [[ -z "$PRODUCT_NAME" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
FRONTEND_DIR="$ROOT_DIR/products/$PRODUCT_NAME/frontend"

if [[ ! -f "$FRONTEND_DIR/package.json" ]]; then
  echo "Frontend package not found: $FRONTEND_DIR/package.json" >&2
  exit 1
fi

if [[ -n "$UI_TARBALL" && ! -f "$UI_TARBALL" ]]; then
  echo "UI tarball not found: $UI_TARBALL" >&2
  exit 1
fi

if [[ -n "$PRODUCT_SHELL_TARBALL" && ! -f "$PRODUCT_SHELL_TARBALL" ]]; then
  echo "Product shell tarball not found: $PRODUCT_SHELL_TARBALL" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d "/tmp/patchhive-${PRODUCT_NAME}-frontend-smoke-XXXXXX")"

cleanup() {
  if [[ "$KEEP_WORKTREE" != true ]]; then
    rm -rf "$WORK_DIR"
  else
    echo "Kept smoke workspace: $WORK_DIR"
  fi
}
trap cleanup EXIT

rsync -a \
  --exclude node_modules \
  --exclude dist \
  --exclude .vite \
  "$FRONTEND_DIR/" "$WORK_DIR/"

node - "$WORK_DIR/package.json" "$UI_TARBALL" "$PRODUCT_SHELL_TARBALL" <<'NODE'
const fs = require("fs");
const [pkgPath, uiTarball, productShellTarball] = process.argv.slice(2);
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
pkg.dependencies ||= {};
if (uiTarball) {
  pkg.dependencies["@patchhivehq/ui"] = `file:${uiTarball}`;
}
if (productShellTarball) {
  pkg.dependencies["@patchhivehq/product-shell"] = `file:${productShellTarball}`;
}
fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
NODE

NPM_CACHE_DIR="${PATCHHIVE_NPM_CACHE_DIR:-/tmp/patchhive-npm-cache}"

echo "Smoke installing frontend dependencies for $PRODUCT_NAME..."
npm --cache "$NPM_CACHE_DIR" install --prefer-online --no-audit --no-fund --prefix "$WORK_DIR"

echo "Smoke building frontend for $PRODUCT_NAME..."
npm --prefix "$WORK_DIR" run build

echo "Packaged frontend dependency smoke passed for $PRODUCT_NAME."
