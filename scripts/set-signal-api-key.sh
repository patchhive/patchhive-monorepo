#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/set-signal-api-key.sh [--stdin] [env-file]

Examples:
  ./scripts/set-signal-api-key.sh
  ./scripts/set-signal-api-key.sh products/signal-hive/.env
  printf '%s' 'your-secret' | ./scripts/set-signal-api-key.sh --stdin

What it does:
  1. Prompts for the SignalHive password you want to keep in a password manager
  2. Hashes it with SHA-256
  3. Writes SIGNAL_API_KEY_HASH=<hash> into the target .env file

Use the original raw password in the frontend login form.
Only the hash is stored in .env.
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

  echo "Could not find sha256sum, shasum, or openssl to hash the SignalHive password." >&2
  exit 1
}

write_hash() {
  local env_file="$1"
  local hash="$2"
  local tmp_file

  mkdir -p "$(dirname "$env_file")"
  touch "$env_file"
  tmp_file="$(mktemp /tmp/signalhive-env-XXXXXX)"

  awk -v key="SIGNAL_API_KEY_HASH" -v hash="$hash" '
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

STDIN_MODE=0

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ "${1:-}" == "--stdin" ]]; then
  STDIN_MODE=1
  shift
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
ENV_FILE="${1:-$ROOT_DIR/products/signal-hive/.env}"

if [[ $# -gt 1 ]]; then
  usage
  exit 1
fi

if [[ "$STDIN_MODE" -eq 1 ]]; then
  IFS= read -r SECRET || true
else
  read -rsp "Enter the SignalHive password to store as SIGNAL_API_KEY_HASH: " SECRET
  printf '\n'

  read -rsp "Confirm the SignalHive password: " CONFIRM
  printf '\n'

  if [[ "$SECRET" != "$CONFIRM" ]]; then
    echo "Passwords did not match. No changes were written." >&2
    exit 1
  fi
fi

if [[ -z "${SECRET:-}" ]]; then
  echo "SignalHive password cannot be empty." >&2
  exit 1
fi

HASH="$(hash_secret "$SECRET")"
write_hash "$ENV_FILE" "$HASH"

echo "Updated $ENV_FILE with SIGNAL_API_KEY_HASH."
echo "Restart SignalHive so it reloads the new hash."
echo "Use the same raw password in the frontend login form."
