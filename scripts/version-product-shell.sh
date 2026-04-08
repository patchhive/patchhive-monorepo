#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

exec "$ROOT_DIR/scripts/version-package.sh" product-shell @patchhivehq/product-shell "$@"
