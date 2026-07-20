#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root_env="${repo_root}/.env"
example_env="${repo_root}/.env.example"
timestamp="$(date +%Y%m%d-%H%M%S)"
backup_dir="${repo_root}/.backups/environment/${timestamp}"
temp_env="$(mktemp "${repo_root}/.env.migrate.XXXXXX")"

cleanup() {
  rm -f "${temp_env}"
}
trap cleanup EXIT

if [[ ! -f "${example_env}" ]]; then
  echo "Missing ${example_env}" >&2
  exit 1
fi

mkdir -p "${backup_dir}"
cp "${example_env}" "${temp_env}"

backup_env() {
  local file="$1"
  [[ -e "${file}" || -L "${file}" ]] || return 0
  local relative="${file#"${repo_root}/"}"
  mkdir -p "${backup_dir}/$(dirname "${relative}")"
  cp -P "${file}" "${backup_dir}/${relative}"
}

read_env_value() {
  local file="$1"
  local wanted="$2"
  [[ -f "${file}" ]] || return 1
  while IFS= read -r line || [[ -n "${line}" ]]; do
    [[ "${line}" =~ ^[[:space:]]*# ]] && continue
    [[ "${line}" == *"="* ]] || continue
    local key="${line%%=*}"
    key="${key#"${key%%[![:space:]]*}"}"
    key="${key%"${key##*[![:space:]]}"}"
    if [[ "${key}" == "${wanted}" ]]; then
      local value="${line#*=}"
      value="${value#"${value%%[![:space:]]*}"}"
      value="${value%"${value##*[![:space:]]}"}"
      [[ -n "${value}" ]] || return 1
      printf '%s' "${value}"
      return 0
    fi
  done < "${file}"
  return 1
}

is_placeholder() {
  local value="$1"
  [[ "${value}" == *"xxxxxxxx"* \
    || "${value}" == "replace-me" \
    || "${value}" == "your-"* \
    || "${value}" == "super-secret" ]]
}

upsert_env_value() {
  local key="$1"
  local value="$2"
  local next
  next="$(mktemp "${repo_root}/.env.upsert.XXXXXX")"
  awk -v wanted="${key}" '
    {
      line = $0
      trimmed = line
      sub(/^[[:space:]]+/, "", trimmed)
      if (index(trimmed, wanted "=") == 1) {
        next
      }
      print line
    }
  ' "${temp_env}" > "${next}"
  printf '\n%s=%s\n' "${key}" "${value}" >> "${next}"
  mv "${next}" "${temp_env}"
}

merge_env_file() {
  local file="$1"
  [[ -f "${file}" ]] || return 0
  while IFS= read -r line || [[ -n "${line}" ]]; do
    [[ "${line}" =~ ^[[:space:]]*# ]] && continue
    [[ "${line}" == *"="* ]] || continue
    local key="${line%%=*}"
    key="${key#"${key%%[![:space:]]*}"}"
    key="${key%"${key##*[![:space:]]}"}"
    case "${key}" in
      BOT_GITHUB_TOKEN|GITHUB_TOKEN|PATCHHIVE_GITHUB_TOKEN_RO|*_GITHUB_TOKEN_RW)
        continue
        ;;
    esac
    [[ "${key}" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
    local value="${line#*=}"
    [[ -n "${value//[[:space:]]/}" ]] || continue
    upsert_env_value "${key}" "${value}"
  done < "${file}"
}

backup_env "${root_env}"
for file in "${repo_root}"/products/*/.env "${repo_root}"/products/*/backend/.env; do
  backup_env "${file}"
done

# Preserve every non-GitHub setting. Root values are loaded first. Legacy
# backend-local files are merged next, then the product-level file wins any
# conflict because it is the operator-facing standalone/Compose configuration.
merge_env_file "${root_env}"
for file in "${repo_root}"/products/*/backend/.env "${repo_root}"/products/*/.env; do
  merge_env_file "${file}"
done

existing_read_token="$(read_env_value "${root_env}" PATCHHIVE_GITHUB_TOKEN_RO || read_env_value "${root_env}" BOT_GITHUB_TOKEN || read_env_value "${root_env}" GITHUB_TOKEN || true)"
if [[ -n "${existing_read_token}" ]] && ! is_placeholder "${existing_read_token}"; then
  upsert_env_value PATCHHIVE_GITHUB_TOKEN_RO "${existing_read_token}"
else
  for file in \
    "${repo_root}/products/signal-hive/.env" \
    "${repo_root}/products/repo-memory/.env" \
    "${repo_root}/products/release-sentry/.env" \
    "${repo_root}/products/flake-sting/.env" \
    "${repo_root}/products/dep-triage/.env" \
    "${repo_root}/products/vuln-triage/.env" \
    "${repo_root}/products/refactor-scout/.env"
  do
    candidate="$(read_env_value "${file}" PATCHHIVE_GITHUB_TOKEN_RO || read_env_value "${file}" BOT_GITHUB_TOKEN || read_env_value "${file}" GITHUB_TOKEN || true)"
    if [[ -n "${candidate}" ]] && ! is_placeholder "${candidate}"; then
      upsert_env_value PATCHHIVE_GITHUB_TOKEN_RO "${candidate}"
      break
    fi
  done
fi

map_write_token() {
  local target_key="$1"
  local source_file="$2"
  local existing
  existing="$(read_env_value "${root_env}" "${target_key}" || true)"
  if [[ -n "${existing}" ]] && ! is_placeholder "${existing}"; then
    upsert_env_value "${target_key}" "${existing}"
    return
  fi
  local candidate
  candidate="$(read_env_value "${source_file}" "${target_key}" || read_env_value "${source_file}" BOT_GITHUB_TOKEN || read_env_value "${source_file}" GITHUB_TOKEN || true)"
  if [[ -n "${candidate}" ]] && ! is_placeholder "${candidate}"; then
    upsert_env_value "${target_key}" "${candidate}"
  fi
}

map_write_token MERGE_KEEPER_GITHUB_TOKEN_RW "${repo_root}/products/merge-keeper/.env"
map_write_token REVIEW_BEE_GITHUB_TOKEN_RW "${repo_root}/products/review-bee/.env"
map_write_token TRUST_GATE_GITHUB_TOKEN_RW "${repo_root}/products/trust-gate/.env"
map_write_token REPO_REAPER_GITHUB_TOKEN_RW "${repo_root}/products/repo-reaper/.env"

mv "${temp_env}" "${root_env}"
chmod 600 "${root_env}"

# Keep standalone product and Docker Compose defaults working while ensuring
# there is one physical environment file in the monorepo.
for product_dir in "${repo_root}"/products/*; do
  [[ -f "${product_dir}/.env.example" ]] || continue
  rm -f "${product_dir}/.env"
  ln -s ../../.env "${product_dir}/.env"
  if [[ -d "${product_dir}/backend" ]]; then
    rm -f "${product_dir}/backend/.env"
    ln -s ../../../.env "${product_dir}/backend/.env"
  fi
done

trap - EXIT
echo "Unified PatchHive environment written to ${root_env}"
echo "Previous environment files backed up under ${backup_dir}"
echo "Product-local .env paths now point to the canonical root file"
