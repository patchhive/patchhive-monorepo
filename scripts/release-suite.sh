#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/release-suite.sh [options]

Common examples:
  ./scripts/release-suite.sh --dry-run
  ./scripts/release-suite.sh --products hive-core,review-bee --skip-publish
  ./scripts/release-suite.sh --packages ui,product-shell --products all

What it does:
  1. Packs shared frontend packages from the monorepo.
  2. Runs packaged-dependency frontend smoke builds for selected products.
  3. Publishes shared npm packages when the local version is not on npm yet.
  4. Syncs shared package mirrors.
  5. Exports selected products to their standalone mirror repositories.
  6. Watches standalone GitHub Actions CI for updated mirrors.

Options:
  --products <all|none|csv>       Products to smoke/export. Default: all.
  --packages <all|none|csv>       Shared packages to pack/publish/sync. Default: all.
  --branch <branch>               Monorepo ref used for publish workflows. Default: current branch.
  --target-branch <branch>        Standalone mirror branch. Default: main.
  --dry-run                       Print intended actions without changing repos or publishing.
  --allow-dirty                   Allow a non-clean worktree for non-dry-run releases.
  --skip-publish                  Do not trigger npm publish workflows.
  --skip-package-mirrors          Do not sync standalone shared package repos.
  --skip-product-smoke            Do not run packaged frontend smoke builds.
  --skip-product-exports          Run selected product smoke checks but do not push product mirrors.
  --skip-products                 Do not select products for smoke or export.
  --skip-ci                       Do not watch standalone CI.
  --ci-timeout-secs <seconds>     Time limit for each CI watch. Default: 3600.
  --no-force-with-lease           Use normal product mirror pushes instead of guarded force-with-lease.
  -h, --help                      Show this help.

Environment:
  PATCHHIVE_NPM_CACHE_DIR         Optional npm cache directory for frontend smoke installs.
EOF
}

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

# shellcheck source=scripts/suite-common.sh
source "$ROOT_DIR/scripts/suite-common.sh"

DRY_RUN=false
ALLOW_DIRTY=false
SKIP_PUBLISH=false
SKIP_PACKAGE_MIRRORS=false
SKIP_PRODUCT_SMOKE=false
SKIP_PRODUCT_EXPORTS=false
SKIP_PRODUCTS=false
SKIP_CI=false
FORCE_WITH_LEASE=true
TARGET_BRANCH="main"
CI_TIMEOUT_SECS=3600
MONOREPO_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
PRODUCT_SELECTOR="all"
PACKAGE_SELECTOR="all"
NPM_CACHE_DIR="${PATCHHIVE_NPM_CACHE_DIR:-/tmp/patchhive-npm-cache}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --products)
      PRODUCT_SELECTOR="${2:-}"
      shift 2
      ;;
    --packages)
      PACKAGE_SELECTOR="${2:-}"
      shift 2
      ;;
    --branch)
      MONOREPO_BRANCH="${2:-}"
      shift 2
      ;;
    --target-branch)
      TARGET_BRANCH="${2:-}"
      shift 2
      ;;
    --ci-timeout-secs)
      CI_TIMEOUT_SECS="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --allow-dirty)
      ALLOW_DIRTY=true
      shift
      ;;
    --skip-publish)
      SKIP_PUBLISH=true
      shift
      ;;
    --skip-package-mirrors)
      SKIP_PACKAGE_MIRRORS=true
      shift
      ;;
    --skip-product-smoke)
      SKIP_PRODUCT_SMOKE=true
      shift
      ;;
    --skip-product-exports)
      SKIP_PRODUCT_EXPORTS=true
      shift
      ;;
    --skip-products)
      SKIP_PRODUCTS=true
      shift
      ;;
    --skip-ci)
      SKIP_CI=true
      shift
      ;;
    --no-force-with-lease)
      FORCE_WITH_LEASE=false
      shift
      ;;
    -*)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
    *)
      echo "Unexpected argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$MONOREPO_BRANCH" || -z "$TARGET_BRANCH" ]]; then
  echo "Branch values cannot be empty." >&2
  exit 1
fi

if [[ -z "$CI_TIMEOUT_SECS" || ! "$CI_TIMEOUT_SECS" =~ ^[0-9]+$ ]]; then
  echo "--ci-timeout-secs must be a positive integer." >&2
  exit 1
fi

select_items() {
  local selector="$1"
  local kind="$2"
  local -n all_ref="$3"
  local -n out_ref="$4"

  out_ref=()
  case "$selector" in
    all)
      out_ref=("${all_ref[@]}")
      ;;
    none|"")
      ;;
    *)
      local split_items=()
      patchhive_split_csv "$selector" split_items
      out_ref=("${split_items[@]}")
      local item
      for item in "${out_ref[@]}"; do
        if ! patchhive_array_contains "$item" "${all_ref[@]}"; then
          echo "Unknown ${kind}: ${item}" >&2
          exit 1
        fi
      done
      ;;
  esac
}

SELECTED_PRODUCTS=()
SELECTED_PACKAGES=()
select_items "$PRODUCT_SELECTOR" "product" PATCHHIVE_PRODUCTS SELECTED_PRODUCTS
select_items "$PACKAGE_SELECTOR" "package" PATCHHIVE_SHARED_PACKAGES SELECTED_PACKAGES

if [[ "$SKIP_PRODUCTS" == true ]]; then
  SELECTED_PRODUCTS=()
fi

if [[ ! "$MONOREPO_BRANCH" =~ ^[A-Za-z0-9._/-]+$ || ! "$TARGET_BRANCH" =~ ^[A-Za-z0-9._/-]+$ ]]; then
  echo "Branch names may only contain letters, numbers, dots, underscores, slashes, and dashes." >&2
  exit 1
fi

if [[ "$DRY_RUN" == false && "$ALLOW_DIRTY" == false ]]; then
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Worktree is dirty. Commit first or rerun with --allow-dirty." >&2
    exit 1
  fi
fi

run() {
  echo "+ $*"
  if [[ "$DRY_RUN" == false ]]; then
    "$@"
  fi
}

run_env() {
  echo "+ env $*"
  if [[ "$DRY_RUN" == false ]]; then
    env "$@"
  fi
}

package_version() {
  local package="$1"
  patchhive_version_from_package_json "$ROOT_DIR/packages/$package/package.json"
}

npm_package_name() {
  local package="$1"
  echo "${PATCHHIVE_PACKAGE_NPM_NAMES[$package]}"
}

wait_for_npm_version() {
  local npm_name="$1"
  local expected_version="$2"
  local deadline=$((SECONDS + 1200))
  local live_version=""

  if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] Would wait for ${npm_name}@${expected_version} on npm."
    return 0
  fi

  echo "Waiting for ${npm_name}@${expected_version} to resolve from npm..."
  while (( SECONDS < deadline )); do
    live_version="$(npm --cache "$NPM_CACHE_DIR" view "${npm_name}@${expected_version}" version 2>/dev/null || true)"
    if [[ "$live_version" == "$expected_version" ]]; then
      echo "${npm_name}@${expected_version} is live."
      return 0
    fi
    sleep 20
  done

  echo "Timed out waiting for ${npm_name}@${expected_version}." >&2
  return 1
}

pack_package() {
  local package="$1"
  local npm_name
  npm_name="$(npm_package_name "$package")"
  echo "Packing ${npm_name}..."
  if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] npm --workspace ${npm_name} pack --pack-destination ${RELEASE_DIR}"
    return 0
  fi
  npm --cache "$NPM_CACHE_DIR" --workspace "$npm_name" pack --pack-destination "$RELEASE_DIR" >/tmp/patchhive-pack-output.txt
  local tarball
  tarball="$(tail -n 1 /tmp/patchhive-pack-output.txt)"
  PACKAGE_TARBALLS[$package]="${RELEASE_DIR}/${tarball}"
  echo "Packed ${PACKAGE_TARBALLS[$package]}"
}

publish_package_if_needed() {
  local package="$1"
  local npm_name version live_version workflow

  npm_name="$(npm_package_name "$package")"
  version="$(package_version "$package")"
  workflow="${PATCHHIVE_PACKAGE_PUBLISH_WORKFLOWS[$package]}"

  if [[ "$SKIP_PUBLISH" == true ]]; then
    echo "Skipping publish for ${npm_name}."
    return 0
  fi

  if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] Would check npm for ${npm_name}@${version}, publish via ${workflow} if missing, then wait for npm."
    return 0
  fi

  live_version="$(npm --cache "$NPM_CACHE_DIR" view "${npm_name}@${version}" version 2>/dev/null || true)"
  if [[ "$live_version" == "$version" ]]; then
    echo "${npm_name}@${version} is already published."
    return 0
  fi

  run gh workflow run "$workflow" --repo patchhive/patchhive2 --ref "$MONOREPO_BRANCH"
  wait_for_npm_version "$npm_name" "$version"
}

sync_package_mirror() {
  local package="$1"
  local remote="${PATCHHIVE_PACKAGE_REMOTES[$package]}"

  if [[ "$SKIP_PACKAGE_MIRRORS" == true ]]; then
    echo "Skipping package mirror sync for ${package}."
    return 0
  fi

  run "$ROOT_DIR/scripts/sync-package-mirror.sh" "$package" "$remote" "$TARGET_BRANCH"
}

remote_sha() {
  local remote="$1"
  local branch="$2"
  git ls-remote "$remote" "refs/heads/${branch}" | awk '{print $1}'
}

watch_repo_ci() {
  local repo="$1"
  local sha="$2"
  local label="$3"
  local deadline=$((SECONDS + CI_TIMEOUT_SECS))
  local run_json run_id

  if [[ "$SKIP_CI" == true ]]; then
    echo "Skipping CI watch for ${label}."
    return 0
  fi

  if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] Would watch ${repo} CI for ${sha} (${label})."
    return 0
  fi

  if [[ -z "$sha" ]]; then
    echo "No remote SHA found for ${label}; cannot watch CI." >&2
    return 1
  fi

  echo "Waiting for ${label} CI run on ${sha}..."
  while (( SECONDS < deadline )); do
    run_json="$(gh run list \
      --repo "$repo" \
      --limit 20 \
      --json databaseId,headSha,status,conclusion,workflowName,displayTitle,createdAt 2>/dev/null || true)"

    run_id="$(node -e '
const runs = JSON.parse(process.argv[1] || "[]");
const sha = process.argv[2];
const run = runs.find((item) => item.headSha === sha);
process.stdout.write(run ? String(run.databaseId) : "");
' "$run_json" "$sha")"

    if [[ -n "$run_id" ]]; then
      gh run watch "$run_id" --repo "$repo" --exit-status
      return $?
    fi

    sleep 15
  done

  echo "Timed out waiting for a GitHub Actions run for ${label} (${sha})." >&2
  return 1
}

smoke_product_frontend() {
  local product="$1"
  local ui_tarball="${PACKAGE_TARBALLS[ui]:-}"
  local shell_tarball="${PACKAGE_TARBALLS[product-shell]:-}"

  if [[ "$SKIP_PRODUCT_SMOKE" == true ]]; then
    echo "Skipping packaged frontend smoke for ${product}."
    return 0
  fi

  if [[ ! -f "$ROOT_DIR/products/$product/frontend/package.json" ]]; then
    echo "No frontend package for ${product}; skipping smoke."
    return 0
  fi

  run_env \
    PATCHHIVE_UI_TARBALL="$ui_tarball" \
    PATCHHIVE_PRODUCT_SHELL_TARBALL="$shell_tarball" \
    "$ROOT_DIR/scripts/smoke-frontend-package-deps.sh" "$product"
}

export_product_mirror() {
  local product="$1"
  local remote="${PATCHHIVE_PRODUCT_REMOTES[$product]}"
  local repo="${PATCHHIVE_PRODUCT_REPOS[$product]}"
  local force_env="PATCHHIVE_EXPORT_FORCE_WITH_LEASE=0"

  if [[ "$FORCE_WITH_LEASE" == true ]]; then
    force_env="PATCHHIVE_EXPORT_FORCE_WITH_LEASE=1"
  fi

  run_env \
    "$force_env" \
    PATCHHIVE_SMOKE_FRONTEND_DEPS=0 \
    "$ROOT_DIR/scripts/export-product.sh" "$product" "$remote" "$TARGET_BRANCH"

  local sha=""
  if [[ "$DRY_RUN" == false ]]; then
    sha="$(remote_sha "$remote" "$TARGET_BRANCH")"
  fi
  watch_repo_ci "$repo" "$sha" "$product"
}

RELEASE_DIR="$(mktemp -d "/tmp/patchhive-release-suite-XXXXXX")"
trap 'rm -rf "$RELEASE_DIR" /tmp/patchhive-pack-output.txt' EXIT
declare -A PACKAGE_TARBALLS=()
CI_FAILURES=()

echo "PatchHive suite release"
echo "  branch: ${MONOREPO_BRANCH}"
echo "  target branch: ${TARGET_BRANCH}"
echo "  packages: ${SELECTED_PACKAGES[*]:-(none)}"
echo "  products: ${SELECTED_PRODUCTS[*]:-(none)}"
echo "  npm cache: ${NPM_CACHE_DIR}"
echo "  dry run: ${DRY_RUN}"
echo

if [[ "$DRY_RUN" == false ]]; then
  mkdir -p "$NPM_CACHE_DIR"
fi

for package in "${SELECTED_PACKAGES[@]}"; do
  pack_package "$package"
done

for product in "${SELECTED_PRODUCTS[@]}"; do
  smoke_product_frontend "$product"
done

for package in "${SELECTED_PACKAGES[@]}"; do
  publish_package_if_needed "$package"
  sync_package_mirror "$package"
  if [[ "$DRY_RUN" == false && "$SKIP_PACKAGE_MIRRORS" == false ]]; then
    package_sha="$(remote_sha "${PATCHHIVE_PACKAGE_REMOTES[$package]}" "$TARGET_BRANCH")"
    if ! watch_repo_ci "${PATCHHIVE_PACKAGE_REPOS[$package]}" "$package_sha" "$package"; then
      CI_FAILURES+=("$package")
    fi
  elif [[ "$DRY_RUN" == true ]]; then
    if ! watch_repo_ci "${PATCHHIVE_PACKAGE_REPOS[$package]}" "" "$package"; then
      CI_FAILURES+=("$package")
    fi
  else
    echo "Skipping CI watch for ${package} because package mirror sync was skipped."
  fi
done

if [[ "$SKIP_PRODUCT_EXPORTS" == true ]]; then
  echo "Skipping product mirror exports."
else
  for product in "${SELECTED_PRODUCTS[@]}"; do
    if ! export_product_mirror "$product"; then
      CI_FAILURES+=("$product")
    fi
  done
fi

if [[ "${#CI_FAILURES[@]}" -gt 0 ]]; then
  echo
  echo "Release completed with CI/watch failures:"
  printf '  - %s\n' "${CI_FAILURES[@]}"
  exit 1
fi

echo
echo "Release suite finished cleanly."
