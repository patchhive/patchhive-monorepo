#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/set-suite-api-key.sh [--stdin] [--stack all|first] [--products slug1,slug2] [--root DIR]

Examples:
  ./scripts/set-suite-api-key.sh
  ./scripts/set-suite-api-key.sh --stack first
  ./scripts/set-suite-api-key.sh --products signal-hive,trust-gate,repo-reaper,hive-core
  printf '%s' 'your-suite-password' | ./scripts/set-suite-api-key.sh --stdin

What it does:
  1. Prompts once for the raw password you want to use across PatchHive products
  2. Hashes it with SHA-256
  3. Writes the matching *_API_KEY_HASH value into each target product's .env file

Use the original raw password in each product's login form.
Only the hash is stored in .env.

This is the easiest way to:
  - keep the same password across products
  - pre-seed auth before first run
  - make subdomain deployments work without localhost-only bootstrap
EOF
}

hash_secret() {
  local value="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s' "$value" | sha256sum | awk '{print $1}'
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    printf '%s' "$value" | shasum -a 256 | awk '{print $1}'
    return
  fi

  if command -v openssl >/dev/null 2>&1; then
    printf '%s' "$value" | openssl dgst -sha256 -r | awk '{print $1}'
    return
  fi

  echo "Could not find sha256sum, shasum, or openssl to hash the suite password." >&2
  exit 1
}

write_hash() {
  local env_file="$1"
  local key="$2"
  local hash="$3"
  local tmp_file

  mkdir -p "$(dirname "$env_file")"
  touch "$env_file"
  tmp_file="$(mktemp /tmp/patchhive-suite-env-XXXXXX)"

  awk -v key="$key" -v hash="$hash" '
    BEGIN { wrote = 0 }
    $0 ~ "^[[:space:]]*#?[[:space:]]*" key "=" {
      if (!wrote) {
        print key "=" hash
        wrote = 1
      }
      next
    }
    { print }
    END {
      if (!wrote) {
        print key "=" hash
      }
    }
  ' "$env_file" > "$tmp_file"

  mv "$tmp_file" "$env_file"
}

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
STDIN_MODE=0
STACK="all"
PRODUCTS_CSV=""

declare -A PRODUCT_HASH_VARS=(
  ["repo-reaper"]="REAPER_API_KEY_HASH"
  ["signal-hive"]="SIGNAL_API_KEY_HASH"
  ["review-bee"]="REVIEW_BEE_API_KEY_HASH"
  ["trust-gate"]="TRUST_API_KEY_HASH"
  ["repo-memory"]="REPO_MEMORY_API_KEY_HASH"
  ["merge-keeper"]="MERGE_KEEPER_API_KEY_HASH"
  ["flake-sting"]="FLAKE_STING_API_KEY_HASH"
  ["dep-triage"]="DEP_TRIAGE_API_KEY_HASH"
  ["vuln-triage"]="VULN_TRIAGE_API_KEY_HASH"
  ["refactor-scout"]="REFACTOR_SCOUT_API_KEY_HASH"
  ["hive-core"]="HIVE_CORE_API_KEY_HASH"
)

ALL_PRODUCTS=(
  "repo-reaper"
  "signal-hive"
  "review-bee"
  "trust-gate"
  "repo-memory"
  "merge-keeper"
  "flake-sting"
  "dep-triage"
  "vuln-triage"
  "refactor-scout"
  "hive-core"
)

FIRST_STACK_PRODUCTS=(
  "signal-hive"
  "trust-gate"
  "repo-reaper"
  "hive-core"
)

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --stdin)
      STDIN_MODE=1
      shift
      ;;
    --stack)
      STACK="${2:-}"
      shift 2
      ;;
    --products)
      PRODUCTS_CSV="${2:-}"
      shift 2
      ;;
    --root)
      ROOT_DIR="${2:-}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$ROOT_DIR" || ! -d "$ROOT_DIR" ]]; then
  echo "Root directory does not exist: $ROOT_DIR" >&2
  exit 1
fi

declare -a TARGET_PRODUCTS=()

if [[ -n "$PRODUCTS_CSV" ]]; then
  IFS=',' read -r -a TARGET_PRODUCTS <<< "$PRODUCTS_CSV"
else
  case "$STACK" in
    all)
      TARGET_PRODUCTS=("${ALL_PRODUCTS[@]}")
      ;;
    first)
      TARGET_PRODUCTS=("${FIRST_STACK_PRODUCTS[@]}")
      ;;
    *)
      echo "Unsupported stack: $STACK (expected 'all' or 'first')." >&2
      exit 1
      ;;
  esac
fi

if [[ "${#TARGET_PRODUCTS[@]}" -eq 0 ]]; then
  echo "No target products were selected." >&2
  exit 1
fi

for product in "${TARGET_PRODUCTS[@]}"; do
  if [[ -z "${PRODUCT_HASH_VARS[$product]:-}" ]]; then
    echo "Unsupported product slug: $product" >&2
    exit 1
  fi
  if [[ ! -d "$ROOT_DIR/products/$product" ]]; then
    echo "Product directory not found: $ROOT_DIR/products/$product" >&2
    exit 1
  fi
done

if [[ "$STDIN_MODE" -eq 1 ]]; then
  IFS= read -r SECRET || true
else
  read -rsp "Enter the suite password to store across selected products: " SECRET
  printf '\n'

  read -rsp "Confirm the suite password: " CONFIRM
  printf '\n'

  if [[ "$SECRET" != "$CONFIRM" ]]; then
    echo "Passwords did not match. No changes were written." >&2
    exit 1
  fi
fi

if [[ -z "${SECRET:-}" ]]; then
  echo "Suite password cannot be empty." >&2
  exit 1
fi

HASH="$(hash_secret "$SECRET")"

for product in "${TARGET_PRODUCTS[@]}"; do
  env_file="$ROOT_DIR/products/$product/.env"
  hash_var="${PRODUCT_HASH_VARS[$product]}"
  write_hash "$env_file" "$hash_var" "$HASH"
done

echo "Updated API-key hashes for:"
for product in "${TARGET_PRODUCTS[@]}"; do
  printf '  - %s (%s)\n' "$product" "${PRODUCT_HASH_VARS[$product]}"
done

echo
echo "Restart the updated products so they reload the new hash."
echo "Use the same raw password in each product login form."
echo "For subdomain deployments, pre-seeding hashes like this avoids localhost-only first-run bootstrap."
