#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/smoke-frontend-package-deps.sh <product-name> [package tarball options] [--keep]

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
  PATCHHIVE_AI_MODELS_TARBALL      Optional local @patchhivehq/ai-models tarball.
  PATCHHIVE_NPM_CACHE_DIR          Optional npm cache directory. Defaults to /tmp/patchhive-npm-cache.
EOF
}

PRODUCT_NAME=""
UI_TARBALL="${PATCHHIVE_UI_TARBALL:-}"
PRODUCT_SHELL_TARBALL="${PATCHHIVE_PRODUCT_SHELL_TARBALL:-}"
AI_MODELS_TARBALL="${PATCHHIVE_AI_MODELS_TARBALL:-}"
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
    --ai-models-tarball)
      AI_MODELS_TARBALL="${2:-}"
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
# shellcheck source=scripts/suite-common.sh
source "$ROOT_DIR/scripts/suite-common.sh"
patchhive_require_inventory_item "product" "$PRODUCT_NAME" "${PATCHHIVE_PRODUCTS[@]}"
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

if [[ -n "$AI_MODELS_TARBALL" && ! -f "$AI_MODELS_TARBALL" ]]; then
  echo "AI models tarball not found: $AI_MODELS_TARBALL" >&2
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

node - "$WORK_DIR/package.json" "$UI_TARBALL" "$PRODUCT_SHELL_TARBALL" "$AI_MODELS_TARBALL" <<'NODE'
const fs = require("fs");
const [pkgPath, uiTarball, productShellTarball, aiModelsTarball] = process.argv.slice(2);
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
pkg.dependencies ||= {};
if (uiTarball) {
  pkg.dependencies["@patchhivehq/ui"] = `file:${uiTarball}`;
}
if (productShellTarball) {
  pkg.dependencies["@patchhivehq/product-shell"] = `file:${productShellTarball}`;
}
if (aiModelsTarball) {
  pkg.dependencies["@patchhivehq/ai-models"] = `file:${aiModelsTarball}`;
}

const unresolvedLocalPackages = Object.entries(pkg.dependencies)
  .filter(([name, version]) => {
    const dependency = String(version);
    return name.startsWith("@patchhive")
      && dependency.startsWith("file:")
      && !dependency.endsWith(".tgz");
  });
if (unresolvedLocalPackages.length > 0) {
  throw new Error(`Missing package tarball overrides for: ${unresolvedLocalPackages.map(([name]) => name).join(", ")}`);
}
fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
NODE

# The monorepo lockfile records workspace links. A standalone install must
# resolve the packed packages above and create its own dependency graph.
rm -f "$WORK_DIR/package-lock.json"

NPM_CACHE_DIR="${PATCHHIVE_NPM_CACHE_DIR:-/tmp/patchhive-npm-cache}"

echo "Smoke installing frontend dependencies for $PRODUCT_NAME..."
npm --cache "$NPM_CACHE_DIR" install --prefer-online --no-audit --no-fund --prefix "$WORK_DIR"

echo "Smoke building frontend for $PRODUCT_NAME..."
npm --prefix "$WORK_DIR" run build

echo "Packaged frontend dependency smoke passed for $PRODUCT_NAME."
