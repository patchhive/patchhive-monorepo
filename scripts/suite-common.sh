#!/usr/bin/env bash

# Shared PatchHive suite inventory for release and drift scripts.
# Keep this file boring and explicit: it is the source of truth for automation.

PATCHHIVE_PRODUCTS=(
  repo-reaper
  signal-hive
  review-bee
  trust-gate
  repo-memory
  merge-keeper
  flake-sting
  dep-triage
  vuln-triage
  refactor-scout
  release-sentry
  hive-core
)

PATCHHIVE_SHARED_PACKAGES=(
  ui
  product-shell
  ai-models
)

PATCHHIVE_TEMPLATES=(
  product-starter
)

declare -gA PATCHHIVE_PRODUCT_TITLES=(
  [repo-reaper]="RepoReaper"
  [signal-hive]="SignalHive"
  [review-bee]="ReviewBee"
  [trust-gate]="TrustGate"
  [repo-memory]="RepoMemory"
  [merge-keeper]="MergeKeeper"
  [flake-sting]="FlakeSting"
  [dep-triage]="DepTriage"
  [vuln-triage]="VulnTriage"
  [refactor-scout]="RefactorScout"
  [release-sentry]="ReleaseSentry"
  [hive-core]="HiveCore"
)

declare -gA PATCHHIVE_PRODUCT_REMOTES=(
  [repo-reaper]="repo-reaper"
  [signal-hive]="signalhive"
  [review-bee]="reviewbee"
  [trust-gate]="trustgate"
  [repo-memory]="repomemory"
  [merge-keeper]="mergekeeper"
  [flake-sting]="flakesting"
  [dep-triage]="deptriage"
  [vuln-triage]="vulntriage"
  [refactor-scout]="refactorscout"
  [release-sentry]="release-sentry"
  [hive-core]="hivecore"
)

declare -gA PATCHHIVE_PRODUCT_REPOS=(
  [repo-reaper]="patchhive/reporeaper"
  [signal-hive]="patchhive/signalhive"
  [review-bee]="patchhive/reviewbee"
  [trust-gate]="patchhive/trustgate"
  [repo-memory]="patchhive/repomemory"
  [merge-keeper]="patchhive/mergekeeper"
  [flake-sting]="patchhive/flakesting"
  [dep-triage]="patchhive/deptriage"
  [vuln-triage]="patchhive/vulntriage"
  [refactor-scout]="patchhive/refactorscout"
  [release-sentry]="patchhive/release-sentry"
  [hive-core]="patchhive/hivecore"
)

declare -gA PATCHHIVE_PRODUCT_FRONTEND_PORTS=(
  [repo-reaper]="5173"
  [signal-hive]="5174"
  [trust-gate]="5175"
  [repo-memory]="5176"
  [review-bee]="5177"
  [merge-keeper]="5178"
  [flake-sting]="5179"
  [dep-triage]="5180"
  [vuln-triage]="5181"
  [refactor-scout]="5182"
  [release-sentry]="5184"
  [hive-core]="5183"
)

declare -gA PATCHHIVE_PRODUCT_BACKEND_PORTS=(
  [repo-reaper]="8000"
  [signal-hive]="8010"
  [trust-gate]="8020"
  [repo-memory]="8030"
  [review-bee]="8040"
  [merge-keeper]="8050"
  [flake-sting]="8060"
  [dep-triage]="8070"
  [refactor-scout]="8090"
  [hive-core]="8100"
  [vuln-triage]="8110"
  [release-sentry]="8120"
)

declare -gA PATCHHIVE_PACKAGE_REMOTES=(
  [ui]="patchhive-ui"
  [product-shell]="product-shell"
  [ai-models]="ai-models"
)

declare -gA PATCHHIVE_PACKAGE_REPOS=(
  [ui]="patchhive/patchhive-ui"
  [product-shell]="patchhive/product-shell"
  [ai-models]="patchhive/ai-models"
)

declare -gA PATCHHIVE_PACKAGE_NPM_NAMES=(
  [ui]="@patchhivehq/ui"
  [product-shell]="@patchhivehq/product-shell"
  [ai-models]="@patchhivehq/ai-models"
)

declare -gA PATCHHIVE_PACKAGE_PUBLISH_WORKFLOWS=(
  [ui]="publish-ui-package.yml"
  [product-shell]="publish-product-shell-package.yml"
  [ai-models]="publish-ai-models-package.yml"
)

declare -gA PATCHHIVE_TEMPLATE_REMOTES=(
  [product-starter]="product-starter"
)

patchhive_array_contains() {
  local needle="$1"
  shift
  local item
  for item in "$@"; do
    [[ "$item" == "$needle" ]] && return 0
  done
  return 1
}

patchhive_split_csv() {
  local raw="$1"
  local -n out_ref="$2"
  local old_ifs="$IFS"
  local item
  out_ref=()
  IFS=","
  read -r -a out_ref <<<"$raw"
  IFS="$old_ifs"
  for item in "${!out_ref[@]}"; do
    out_ref[$item]="${out_ref[$item]#"${out_ref[$item]%%[![:space:]]*}"}"
    out_ref[$item]="${out_ref[$item]%"${out_ref[$item]##*[![:space:]]}"}"
  done
}

patchhive_version_from_package_json() {
  local package_json="$1"
  node -e 'const fs=require("fs"); const p=JSON.parse(fs.readFileSync(process.argv[1],"utf8")); console.log(p.version || "");' "$package_json"
}
