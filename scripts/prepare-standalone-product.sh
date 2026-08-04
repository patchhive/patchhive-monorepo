#!/usr/bin/env bash

set -euo pipefail

EXPORT_ROOT="${1:-}"
PRODUCT_NAME="${2:-}"
SOURCE_ROOT="${PATCHHIVE_SOURCE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"

if [[ -z "$EXPORT_ROOT" || -z "$PRODUCT_NAME" || ! -d "$EXPORT_ROOT" ]]; then
  echo "Usage: $0 <export-root> <product-name>" >&2
  exit 1
fi

if [[ -f "$EXPORT_ROOT/backend/Cargo.toml" ]]; then
  rm -rf "$EXPORT_ROOT/shared-crates"
  mkdir -p "$EXPORT_ROOT/shared-crates"
  for crate in \
    patchhive-product-core \
    patchhive-github-pr \
    patchhive-github-data \
    patchhive-github-security; do
    mkdir -p "$EXPORT_ROOT/shared-crates/$crate"
    rsync -a --exclude target/ "$SOURCE_ROOT/crates/$crate/" "$EXPORT_ROOT/shared-crates/$crate/"
  done
  "$SOURCE_ROOT/scripts/prepare-standalone-cargo-manifest.sh" "$EXPORT_ROOT/backend/Cargo.toml"

  cat >"$EXPORT_ROOT/backend/Dockerfile" <<EOF
# syntax=docker/dockerfile:1.7

ARG RUST_IMAGE=rust:1.97.1-bookworm@sha256:77fac8b98f9f46062bb680b6d25d5bcaabfc400143952ebc572e924bcbedc3fa
ARG RUNTIME_IMAGE=debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818

FROM \${RUST_IMAGE} AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev git && rm -rf /var/lib/apt/lists/*
COPY shared-crates ./shared-crates
COPY backend ./backend
WORKDIR /app/backend
RUN cargo build --release --locked

FROM \${RUNTIME_IMAGE}
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates git libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/backend/target/release/$PRODUCT_NAME /usr/local/bin/$PRODUCT_NAME
RUN groupadd --gid 10001 patchhive && useradd --uid 10001 --gid patchhive --create-home --shell /usr/sbin/nologin patchhive
WORKDIR /app
USER 10001:10001
CMD ["$PRODUCT_NAME"]
EOF
fi

if [[ ! -f "$EXPORT_ROOT/frontend/package.json" ]]; then
  exit 0
fi

node - "$SOURCE_ROOT" "$EXPORT_ROOT" "$PRODUCT_NAME" <<'NODE'
const fs = require("fs");
const path = require("path");

const [sourceRoot, exportRoot, productName] = process.argv.slice(2);
const packageMap = {
  "@patchhivehq/ui": "ui",
  "@patchhivehq/product-shell": "product-shell",
  "@patchhivehq/ai-models": "ai-models",
};

const frontendPackagePath = path.join(exportRoot, "frontend/package.json");
const frontendPackage = JSON.parse(fs.readFileSync(frontendPackagePath, "utf8"));
frontendPackage.dependencies ||= {};
for (const [packageName, packageDir] of Object.entries(packageMap)) {
  if (!frontendPackage.dependencies[packageName]) continue;
  const sharedPackage = JSON.parse(
    fs.readFileSync(path.join(sourceRoot, "packages", packageDir, "package.json"), "utf8"),
  );
  frontendPackage.dependencies[packageName] = `^${sharedPackage.version}`;
}
fs.writeFileSync(frontendPackagePath, `${JSON.stringify(frontendPackage, null, 2)}\n`);

const dockerfile = `# syntax=docker/dockerfile:1.7

ARG NODE_IMAGE=node:24-alpine@sha256:d32cdf619f63fe0471182d08996dd516c6275bb5fd31ae06e55a570bd9e1ad43
ARG NGINX_IMAGE=nginxinc/nginx-unprivileged:stable-alpine@sha256:44e36330f74d4f3a1d4e222acca9e23b401fb87811a7597024502bb759c4dd49

FROM \${NODE_IMAGE} AS builder
WORKDIR /app
COPY frontend/package*.json ./
RUN npm ci --prefer-online --no-audit --no-fund
COPY frontend/. .
ARG VITE_API_URL=/api
ENV VITE_API_URL=$VITE_API_URL
RUN npm run build

FROM \${NGINX_IMAGE}
COPY --from=builder /app/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080
`;
fs.writeFileSync(path.join(exportRoot, "frontend/Dockerfile"), dockerfile);

const composePath = path.join(exportRoot, "docker-compose.yml");
if (fs.existsSync(composePath)) {
  const lines = fs.readFileSync(composePath, "utf8").split("\n");
  const backendIndex = lines.findIndex((line) => line === "  backend:");
  if (backendIndex >= 0) {
    const buildIndex = lines.findIndex((line, index) => index > backendIndex && line.startsWith("    build:"));
    if (buildIndex >= 0) {
      let endIndex = buildIndex + 1;
      if (lines[buildIndex] === "    build:") {
        while (endIndex < lines.length && (lines[endIndex].startsWith("      ") || lines[endIndex].trim() === "")) {
          endIndex += 1;
        }
      }
      lines.splice(
        buildIndex,
        endIndex - buildIndex,
        "    build:",
        "      context: .",
        "      dockerfile: backend/Dockerfile",
      );
    }
  }
  const frontendIndex = lines.findIndex((line) => line === "  frontend:");
  if (frontendIndex >= 0) {
    const buildIndex = lines.findIndex((line, index) => index > frontendIndex && line.startsWith("    build:"));
    if (buildIndex >= 0 && lines[buildIndex] === "    build:") {
      let endIndex = buildIndex + 1;
      while (endIndex < lines.length && (lines[endIndex].startsWith("      ") || lines[endIndex].trim() === "")) {
        endIndex += 1;
      }
      lines.splice(
        buildIndex,
        endIndex - buildIndex,
        "    build:",
        "      context: .",
        "      dockerfile: frontend/Dockerfile",
      );
      fs.writeFileSync(composePath, lines.join("\n"));
    }
  }
}

const workflowPath = path.join(exportRoot, ".github/workflows/ci.yml");
if (fs.existsSync(workflowPath)) {
  let workflow = fs.readFileSync(workflowPath, "utf8");
  const original = workflow;
  workflow = workflow
    .replace(
      "          - service: backend\n            context: backend\n",
      "          - service: backend\n            context: .\n            dockerfile: backend/Dockerfile\n",
    )
    .replace(
      "          - service: frontend\n            context: frontend\n",
      "          - service: frontend\n            context: .\n            dockerfile: frontend/Dockerfile\n",
    )
    .replace(
      "          context: \${{ matrix.context }}\n",
      "          context: \${{ matrix.context }}\n          file: \${{ matrix.dockerfile }}\n",
    )
    .replace("npm install --prefer-online", "npm ci --prefer-online");
  if (workflow === original || !workflow.includes("file: \${{ matrix.dockerfile }}")) {
    throw new Error("Could not rewrite standalone CI Docker contexts safely.");
  }
  fs.writeFileSync(workflowPath, workflow);
}
NODE

rm -f "$EXPORT_ROOT/frontend/package-lock.json"
npm --prefix "$EXPORT_ROOT/frontend" install \
  --package-lock-only \
  --ignore-scripts \
  --prefer-online \
  --no-audit \
  --no-fund

echo "Prepared portable standalone files for $PRODUCT_NAME"
