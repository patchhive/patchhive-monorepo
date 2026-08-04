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
FROM rust:1.87-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY shared-crates ./shared-crates
COPY backend ./backend
WORKDIR /app/backend
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/backend/target/release/$PRODUCT_NAME /usr/local/bin/$PRODUCT_NAME
RUN useradd --system --create-home --uid 10001 patchhive && chown patchhive:patchhive /usr/local/bin/$PRODUCT_NAME
WORKDIR /app
USER patchhive
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

const dockerfile = `FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm install --prefer-online --no-audit --no-fund
COPY . .
ARG VITE_API_URL=/api
ENV VITE_API_URL=$VITE_API_URL
RUN npm run build

FROM nginxinc/nginx-unprivileged:stable-alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
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
      lines.splice(buildIndex, endIndex - buildIndex, "    build: ./frontend");
      fs.writeFileSync(composePath, lines.join("\n"));
    }
  }
}
NODE

rm -f "$EXPORT_ROOT/frontend/package-lock.json"

echo "Prepared portable standalone files for $PRODUCT_NAME"
