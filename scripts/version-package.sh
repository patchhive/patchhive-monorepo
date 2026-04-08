#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/version-package.sh <package-dir> <package-name> <patch|minor|major|x.y.z> [--dry-run]

Examples:
  ./scripts/version-package.sh ui @patchhivehq/ui patch
  ./scripts/version-package.sh product-shell @patchhivehq/product-shell minor
  ./scripts/version-package.sh product-shell @patchhivehq/product-shell 0.2.0
  ./scripts/version-package.sh ui @patchhivehq/ui patch --dry-run

What it updates:
  - packages/<package-dir>/package.json
  - package-lock.json workspace metadata
  - workspace package.json files that depend on the package name you pass
EOF
}

DRY_RUN=false
POSITIONAL=()

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
      POSITIONAL+=("$arg")
      ;;
  esac
done

set -- "${POSITIONAL[@]}"

PACKAGE_DIR="${1:-}"
PACKAGE_NAME="${2:-}"
SPEC="${3:-}"

if [[ -z "$PACKAGE_DIR" || -z "$PACKAGE_NAME" || -z "$SPEC" ]]; then
  usage
  exit 1
fi

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

node - "$ROOT_DIR" "$PACKAGE_DIR" "$PACKAGE_NAME" "$SPEC" "$DRY_RUN" <<'NODE'
const fs = require("fs");
const path = require("path");

const [rootDir, packageDir, packageName, spec, dryRunFlag] = process.argv.slice(2);
const dryRun = dryRunFlag === "true";
const pkgPath = path.join(rootDir, `packages/${packageDir}/package.json`);
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

if (lock.packages[`packages/${packageDir}`]) {
  lock.packages[`packages/${packageDir}`].version = newVersion;
}

const dependencyKeys = ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"];
const packageJsonFiles = walkPackageJsonFiles(rootDir)
  .filter((filePath) => filePath !== pkgPath);

const touchedManifests = [];

for (const filePath of packageJsonFiles) {
  const json = readJson(filePath);
  let touched = false;

  for (const key of dependencyKeys) {
    if (json[key] && json[key][packageName]) {
      json[key][packageName] = `^${newVersion}`;
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
    if (pkgMeta[key] && pkgMeta[key][packageName]) {
      pkgMeta[key][packageName] = `^${newVersion}`;
    }
  }
}

console.log(`${packageName} ${currentVersion} -> ${newVersion}`);
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
