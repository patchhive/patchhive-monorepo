#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${PATCHHIVE_LAUNCHER_URL:-http://127.0.0.1:8210}"

echo "patchhive-launcher smoke: ${BASE_URL}"
echo

echo "health:"
curl -fsSL "${BASE_URL}/health"
echo
echo

echo "products:"
curl -fsSL "${BASE_URL}/products"
echo
echo

echo "first stack:"
curl -fsSL "${BASE_URL}/stacks/first"
echo
echo

if [[ "${PATCHHIVE_SMOKE_START:-0}" == "1" ]]; then
  echo "starting first stack:"
  curl -fsSL \
    -X POST \
    -H "Content-Type: application/json" \
    -d "{}" \
    "${BASE_URL}/stacks/first/start"
  echo
else
  echo "skipped start. Set PATCHHIVE_SMOKE_START=1 to run docker compose for the first stack."
fi
