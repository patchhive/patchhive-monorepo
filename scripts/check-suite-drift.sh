#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

# shellcheck source=scripts/suite-common.sh
source "$ROOT_DIR/scripts/suite-common.sh"

failures=0

fail() {
  echo "drift: $*" >&2
  failures=$((failures + 1))
}

require_file() {
  local path="$1"
  [[ -f "$path" ]] || fail "missing file: $path"
}

require_dir() {
  local path="$1"
  [[ -d "$path" ]] || fail "missing directory: $path"
}

require_contains() {
  local path="$1"
  local needle="$2"
  local label="${3:-$needle}"
  if [[ ! -f "$path" ]]; then
    fail "missing file: $path"
    return
  fi
  if ! grep -Fq "$needle" "$path"; then
    fail "$path missing ${label}"
  fi
}

json_field() {
  local path="$1"
  local field="$2"
  node -e '
const fs = require("fs");
const pkg = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const field = process.argv[2].split(".");
let value = pkg;
for (const part of field) value = value?.[part];
process.stdout.write(value || "");
' "$path" "$field"
}

check_frontend_dependencies() {
  local package_json="$1"
  local label="$2"
  local expected_ui="^$(patchhive_version_from_package_json "$ROOT_DIR/packages/ui/package.json")"
  local expected_shell="^$(patchhive_version_from_package_json "$ROOT_DIR/packages/product-shell/package.json")"
  local actual_ui actual_ui_v2 actual_shell

  actual_ui="$(json_field "$package_json" "dependencies.@patchhivehq/ui")"
  actual_ui_v2="$(json_field "$package_json" "dependencies.@patchhivehq/ui-v2")"
  actual_shell="$(json_field "$package_json" "dependencies.@patchhivehq/product-shell")"

  if [[ -n "$actual_ui_v2" ]]; then
    [[ -n "$actual_shell" ]] || fail "$label uses @patchhivehq/ui-v2 but is missing @patchhivehq/product-shell"
    return
  fi

  [[ "$actual_ui" == "$expected_ui" ]] || fail "$label uses @patchhivehq/ui ${actual_ui:-<missing>}, expected ${expected_ui}"
  [[ "$actual_shell" == "$expected_shell" ]] || fail "$label uses @patchhivehq/product-shell ${actual_shell:-<missing>}, expected ${expected_shell}"
}

check_theme_inventory() {
  local output
  output="$(node - "$ROOT_DIR/packages/ui/src/theme.js" "${PATCHHIVE_PRODUCTS[@]}" <<'NODE'
const fs = require("fs");
const [themePath, ...products] = process.argv.slice(2);
const source = fs.readFileSync(themePath, "utf8");
const entries = new Map();
const regex = /"([^"]+)":\s*\{\s*"--accent":\s*"([^"]+)"/g;
let match;
while ((match = regex.exec(source))) {
  entries.set(match[1], match[2].toLowerCase());
}
for (const product of products) {
  if (!entries.has(product)) {
    console.log(`theme missing product key ${product}`);
  }
}
const seen = new Map();
for (const product of products) {
  const accent = entries.get(product);
  if (!accent) continue;
  if (seen.has(accent)) {
    console.log(`theme accent ${accent} is shared by ${seen.get(accent)} and ${product}`);
  } else {
    seen.set(accent, product);
  }
}
NODE
)"
  if [[ -n "$output" ]]; then
    while IFS= read -r line; do
      fail "$line"
    done <<<"$output"
  fi
}

check_product() {
  local product="$1"
  local title="${PATCHHIVE_PRODUCT_TITLES[$product]}"
  local repo="${PATCHHIVE_PRODUCT_REPOS[$product]}"
  local frontend_port="${PATCHHIVE_PRODUCT_FRONTEND_PORTS[$product]}"
  local backend_port="${PATCHHIVE_PRODUCT_BACKEND_PORTS[$product]}"
  local product_dir="products/$product"
  local doc_path="docs/products/$product.md"
  local readme_path="$product_dir/README.md"
  local workflow_path="$product_dir/.github/workflows/ci.yml"
  local frontend_dir="$product_dir/frontend"
  local frontend_kind="v1"

  if [[ ! -f "$frontend_dir/package.json" && -f "$product_dir/frontend-v2/package.json" ]]; then
    frontend_dir="$product_dir/frontend-v2"
    frontend_kind="v2"
  fi

  require_dir "$product_dir"
  require_file "$readme_path"
  require_file "$doc_path"
  require_file "$product_dir/.env.example"
  require_file "$product_dir/docker-compose.yml"
  require_file "$product_dir/backend/Cargo.toml"
  require_file "$product_dir/backend/Dockerfile"
  require_file "$frontend_dir/package.json"
  require_file "$frontend_dir/Dockerfile"
  require_file "$workflow_path"

  require_contains "$readme_path" "# ${title} by PatchHive" "product title"
  require_contains "$readme_path" "docs/products/${product}.md" "product docs link"
  require_contains "$readme_path" "$repo" "standalone repository link"
  require_contains "$readme_path" "Frontend: \`http://localhost:${frontend_port}\`" "frontend port ${frontend_port}"
  require_contains "$readme_path" "Backend: \`http://localhost:${backend_port}\`" "backend port ${backend_port}"

  require_contains "$doc_path" "# ${title}" "docs title"
  require_contains "$doc_path" "cd products/${product}" "local dev product path"
  require_contains "$doc_path" "Frontend: \`http://localhost:${frontend_port}\`" "docs frontend port ${frontend_port}"
  require_contains "$doc_path" "Backend: \`http://localhost:${backend_port}\`" "docs backend port ${backend_port}"
  require_contains "$doc_path" "$repo" "docs standalone repository link"

  require_contains "README.md" "$repo" "root README entry for ${product}"
  require_contains "docs/products/README.md" "${product}.md" "product docs index entry for ${product}"
  require_contains "packages/ui/src/theme.js" "\"${product}\":" "theme key ${product}"

  if [[ "$frontend_kind" == "v2" ]]; then
    if command -v rg >/dev/null 2>&1; then
      if ! rg -q "productKey=[\"']${product}[\"']" "$frontend_dir/src"; then
        fail "$product frontend-v2 does not declare productKey ${product}"
      fi
    elif ! grep -R -Eq "productKey=[\"']${product}[\"']" "$frontend_dir/src"; then
      fail "$product frontend-v2 does not declare productKey ${product}"
    fi
  elif command -v rg >/dev/null 2>&1; then
    if ! rg -q "applyTheme\\([\"']${product}[\"']" "$frontend_dir/src"; then
      fail "$product frontend does not apply theme ${product}"
    fi
  elif ! grep -R -Eq "applyTheme\\([\"']${product}[\"']" "$frontend_dir/src"; then
    fail "$product frontend does not apply theme ${product}"
  fi

  check_frontend_dependencies "$frontend_dir/package.json" "$product"

  require_contains "$workflow_path" "FORCE_JAVASCRIPT_ACTIONS_TO_NODE24" "Node 24 action shim"
  require_contains "$workflow_path" "uses: actions/checkout@v5" "checkout v5"
  require_contains "$workflow_path" "uses: actions/setup-node@v5" "setup-node v5"
  require_contains "$workflow_path" "node-version: 24" "Node 24"
  require_contains "$workflow_path" "uses: docker/build-push-action@v6" "Docker build action v6"

  require_contains "$product_dir/docker-compose.yml" "\"${backend_port}:8000\"" "docker backend port mapping"
  require_contains "$product_dir/docker-compose.yml" "\"${frontend_port}:8080\"" "docker frontend port mapping"
}

check_template() {
  local template="templates/product-starter/scaffold"
  require_file "$template/README.md"
  require_file "$template/frontend/package.json"
  require_file "$template/.github/workflows/ci.yml"
  check_frontend_dependencies "$template/frontend/package.json" "product-starter scaffold"
  require_contains "$template/frontend/src/App.jsx" "ProductSessionGate" "template shared session gate"
  require_contains "$template/frontend/src/App.jsx" "ProductAppFrame" "template shared app frame"
  require_contains "$template/.github/workflows/ci.yml" "uses: actions/checkout@v5" "template checkout v5"
  require_contains "$template/.github/workflows/ci.yml" "uses: actions/setup-node@v5" "template setup-node v5"
  require_contains "$template/.github/workflows/ci.yml" "node-version: 24" "template Node 24"
}

check_release_docs() {
  require_file "scripts/release-suite.sh"
  require_file "scripts/smoke-frontend-package-deps.sh"
  require_file "docs/release-checklist.md"
  require_file "docs/product-export-workflow.md"
  require_contains "README.md" "npm run release:suite" "suite release command"
  require_contains "README.md" "npm run check:suite-drift" "suite drift command"
  require_contains "docs/release-checklist.md" "./scripts/release-suite.sh" "suite release script"
  require_contains "docs/product-export-workflow.md" "PATCHHIVE_EXPORT_FORCE_WITH_LEASE" "force-with-lease export option"
}

check_theme_inventory
for product in "${PATCHHIVE_PRODUCTS[@]}"; do
  check_product "$product"
done
check_template
check_release_docs

if [[ "$failures" -gt 0 ]]; then
  echo
  echo "Suite drift check failed with ${failures} issue(s)." >&2
  exit 1
fi

echo "Suite drift check passed."
