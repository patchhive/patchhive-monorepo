#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/version-ui.sh <patch|minor|major|x.y.z> [--dry-run]

Examples:
  ./scripts/version-ui.sh patch
  ./scripts/version-ui.sh minor
  ./scripts/version-ui.sh 0.2.0
  ./scripts/version-ui.sh patch --dry-run

What it updates:
  - packages/ui/package.json
  - package-lock.json workspace metadata
  - workspace package.json files that depend on @patchhivehq/ui
EOF
}

DRY_RUN=false
SPEC=""

for arg in "$@"; do
  case "$arg" in
    -h|--help)
      usage
      exit 0
      ;;
    --dry-run)
      DRY_RUN=true
      ;;
    *)
      if [[ -z "$SPEC" ]]; then
        SPEC="$arg"
      else
        echo "Unexpected argument: $arg" >&2
        usage
        exit 1
      fi
      ;;
  esac
done

if [[ -z "$SPEC" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

node - "$ROOT_DIR" "$SPEC" "$DRY_RUN" <<'NODE'
const fs = require("fs");
const path = require("path");

const [rootDir, spec, dryRunFlag] = process.argv.slice(2);
const dryRun = dryRunFlag === "true";
const uiName = "@patchhivehq/ui";
const pkgPath = path.join(rootDir, "packages/ui/package.json");
const lockPath = path.join(rootDir, "package-lock.json");

function parseSemver(value) {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(value);
  if (!match) {
    throw new Error(`Expected semver x.y.z, received "${value}"`);
  }
  return match.slice(1).map((part) => Number(part));
}

function nextVersion(current, requested) {
  const [major, minor, patch] = parseSemver(current);
  if (requested === "patch") return `${major}.${minor}.${patch + 1}`;
  if (requested === "minor") return `${major}.${minor + 1}.0`;
  if (requested === "major") return `${major + 1}.0.0`;
  parseSemver(requested);
  return requested;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, data) {
  fs.writeFileSync(filePath, `${JSON.stringify(data, null, 2)}\n`);
}

function walkPackageJsonFiles(startDir) {
  const results = [];
  const entries = fs.readdirSync(startDir, { withFileTypes: true });

  for (const entry of entries) {
    if (entry.name === "node_modules" || entry.name === "dist" || entry.name === "target" || entry.name === ".git") {
      continue;
    }

    const fullPath = path.join(startDir, entry.name);
    if (entry.isDirectory()) {
      results.push(...walkPackageJsonFiles(fullPath));
      continue;
    }

    if (entry.isFile() && entry.name === "package.json") {
      results.push(fullPath);
    }
  }

  return results;
}

const pkg = readJson(pkgPath);
const lock = readJson(lockPath);
const currentVersion = pkg.version;
const newVersion = nextVersion(currentVersion, spec);

if (currentVersion === newVersion) {
  throw new Error(`Version is already ${currentVersion}`);
}

pkg.version = newVersion;
lock.packages["packages/ui"].version = newVersion;

const dependencyKeys = ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"];
const packageJsonFiles = walkPackageJsonFiles(rootDir)
  .filter((filePath) => filePath !== pkgPath);

const touchedManifests = [];

for (const filePath of packageJsonFiles) {
  const json = readJson(filePath);
  let touched = false;

  for (const key of dependencyKeys) {
    if (json[key] && json[key][uiName]) {
      json[key][uiName] = `^${newVersion}`;
      touched = true;
    }
  }

  if (touched) {
    touchedManifests.push(path.relative(rootDir, filePath));
    if (!dryRun) {
      writeJson(filePath, json);
    }
  }
}

for (const pkgMeta of Object.values(lock.packages)) {
  if (!pkgMeta || typeof pkgMeta !== "object") continue;
  for (const key of dependencyKeys) {
    if (pkgMeta[key] && pkgMeta[key][uiName]) {
      pkgMeta[key][uiName] = `^${newVersion}`;
    }
  }
}

console.log(`@patchhivehq/ui ${currentVersion} -> ${newVersion}`);
if (touchedManifests.length) {
  console.log("Updated dependents:");
  for (const filePath of touchedManifests) {
    console.log(`- ${filePath}`);
  }
}

if (!dryRun) {
  writeJson(pkgPath, pkg);
  writeJson(lockPath, lock);
}
NODE
