#!/usr/bin/env bash

set -euo pipefail

MODE="vendored"
if [[ "${1:-}" == "--git-mirrors" ]]; then
  MODE="git-mirrors"
  shift
fi

MANIFEST_PATH="${1:-}"
if [[ -z "$MANIFEST_PATH" || ! -f "$MANIFEST_PATH" ]]; then
  echo "Usage: $0 <Cargo.toml>" >&2
  exit 1
fi

# Product exports carry an exact snapshot of the shared crates they were tested
# with. Standalone crate mirrors still use the public git mirrors because they
# cannot contain sibling crate snapshots.
node - "$MANIFEST_PATH" "$MODE" <<'NODE'
const fs = require("fs");

const manifestPath = process.argv[2];
const mode = process.argv[3];
const gitDependencies = {
  "patchhive-product-core": {
    git: "https://github.com/patchhive/patchhive-product-core.git",
    rev: "60890883e057ac5e18353dcb0413c002842a2eba",
  },
  "patchhive-github-pr": {
    git: "https://github.com/patchhive/patchhive-github-pr.git",
    rev: "18d9e3517fa9daca2b8ddfac4ce692de7d1d0aa6",
  },
  "patchhive-github-data": {
    git: "https://github.com/patchhive/patchhive-github-data.git",
    rev: "fd7fabdabeea87d25965c26d59f5bc82d7e9f311",
  },
  "patchhive-github-security": {
    git: "https://github.com/patchhive/patchhive-github-security.git",
    rev: "f5ed2af1966ba6ec584585078faaaf6877f74ae3",
  },
};

let source = fs.readFileSync(manifestPath, "utf8");
for (const [name, dependency] of Object.entries(gitDependencies)) {
  const pattern = new RegExp(`^${name.replaceAll("-", "\\-")}\\s*=\\s*\\{[^\\n]*\\}$`, "m");
  if (!pattern.test(source)) continue;
  const replacement = mode === "git-mirrors"
    ? `${name} = { git = "${dependency.git}", rev = "${dependency.rev}", version = "0.1.0" }`
    : `${name} = { path = "../shared-crates/${name}" }`;
  source = source.replace(
    pattern,
    replacement,
  );
}

const unresolved = source
  .split("\n")
  .filter((line) => {
    if (!/^patchhive-(product-core|github-(pr|data|security))\s*=/.test(line)) return false;
    return mode === "git-mirrors"
      ? /\bpath\s*=/.test(line)
      : !line.includes('path = "../shared-crates/');
  });
if (unresolved.length > 0) {
  throw new Error(`Unsupported PatchHive dependency shape:\n${unresolved.join("\n")}`);
}
fs.writeFileSync(manifestPath, source);
NODE
