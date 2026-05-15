# Release Checklist

PatchHive releases are driven from the monorepo first, then mirrored into standalone repositories where appropriate.

## Suite Release Automation

For normal suite releases, prefer the shared runner:

```bash
./scripts/release-suite.sh
```

Start with a dry run when package versions, product exports, or CI state changed:

```bash
./scripts/release-suite.sh --dry-run
```

Useful scoped runs:

```bash
./scripts/release-suite.sh --products hive-core --skip-publish
./scripts/release-suite.sh --products hive-core --skip-publish --skip-product-exports
./scripts/release-suite.sh --packages ui,product-shell --skip-products
./scripts/release-suite.sh --products review-bee,merge-keeper --skip-publish --skip-package-mirrors
```

The runner packs shared frontend packages, runs exported-product dependency smokes, publishes missing npm versions through the GitHub workflows, syncs package mirrors, exports selected product mirrors, and watches standalone CI. Use `--skip-product-exports` when you only want the packaged frontend smoke path. It uses guarded `--force-with-lease` product mirror pushes by default because those repositories are exported mirrors of the monorepo.

Before opening or merging release work, run:

```bash
./scripts/check-suite-drift.sh
```

The monorepo `Suite Drift` workflow runs the same guard in CI.

## RepoReaper Release

1. Confirm the monorepo is clean and `products/repo-reaper` is in the state you want to ship.
2. Run local verification:

```bash
cd products/repo-reaper/backend && cargo check --locked
cd ../frontend && npm run build
```

3. Commit and push the monorepo changes.
4. Export the standalone repo:

```bash
./scripts/export-product.sh repo-reaper repo-reaper main
```

5. Confirm the standalone `RepoReaper CI` workflow is green.
6. Tag the release:

```bash
./scripts/tag-release.sh repo-reaper v0.1.0
```

That creates:

- monorepo tag `repo-reaper/v0.1.0`
- standalone repo tag `v0.1.0` in `patchhive/reporeaper`

## UI Package Release

1. Bump the package version:

```bash
npm run version:ui -- patch
```

2. Review the changed files:

- `packages/ui/package.json`
- `package-lock.json`
- any dependents updated to the new `^version`

3. Commit and push the monorepo changes.
4. Run the `Publish UI Package` workflow in `patchhive/patchhive2`.
5. Sync the standalone mirror:

```bash
npm run mirror:ui
```

6. Confirm `patchhive-ui CI` is green.
7. Tag the release:

```bash
./scripts/tag-release.sh ui v0.1.0
```

That creates:

- monorepo tag `ui/v0.1.0`
- standalone repo tag `v0.1.0` in `patchhive/patchhive-ui`

## Product Shell Package Release

1. Bump the package version:

```bash
npm run version:product-shell -- patch
```

2. Review the changed files:

- `packages/product-shell/package.json`
- `package-lock.json`
- any dependents updated to the new `^version`

3. Commit and push the monorepo changes.
4. Run the `Publish Product Shell Package` workflow in `patchhive/patchhive2`.
5. Sync the standalone mirror:

```bash
npm run mirror:product-shell
```

6. Confirm `product-shell CI` is green.
7. Tag the release:

```bash
./scripts/tag-release.sh product-shell v0.1.0
```

That creates:

- monorepo tag `product-shell/v0.1.0`
- standalone repo tag `v0.1.0` in `patchhive/product-shell`

## AI Local Release

1. Confirm the monorepo is clean and `packages/ai-local` is in the state you want to ship.
2. Run local verification:

```bash
cd packages/ai-local
node --check src/cli.js
node --check adapters/codex/index.js
node --check adapters/copilot/index.js
cd rust-gateway && cargo check --locked
```

3. Commit and push the monorepo changes.
4. Sync the standalone mirror:

```bash
./scripts/sync-package-mirror.sh ai-local patchhive-ai-local main
```

5. Confirm `patchhive-ai-local CI` is green.
6. Tag the release:

```bash
./scripts/tag-release.sh ai-local v0.1.0
```

That creates:

- monorepo tag `ai-local/v0.1.0`
- standalone repo tag `v0.1.0` in `patchhive/patchhive-ai-local`

## Product Core Crate Release

1. Confirm the monorepo is clean and `crates/patchhive-product-core` is in the state you want to ship.
2. Run local verification:

```bash
cd crates/patchhive-product-core && cargo check --locked
cd ../../products/repo-reaper/backend && cargo check
cd ../../signal-hive/backend && cargo check
```

3. Commit and push the monorepo changes.
4. Sync the standalone mirror:

```bash
npm run mirror:product-core
```

5. Confirm `patchhive-product-core CI` is green.
6. Tag the release:

```bash
./scripts/tag-release.sh patchhive-product-core v0.1.0
```

That creates:

- monorepo tag `patchhive-product-core/v0.1.0`
- standalone repo tag `v0.1.0` in `patchhive/patchhive-product-core`

## Notes

- The monorepo remains the source of truth.
- Product repos can keep subtree-style history.
- Shared package/service repos should prefer clean mirror syncs.
- Shared crate repos should prefer clean mirror syncs too.
- The monorepo's default `.cargo/config.toml` intentionally does not load local shared-crate patches. Use `cargo --config .cargo/local-patches.toml ...` only while actively testing local shared-crate edits against products; exported repos and normal product commands should enforce strict standalone lockfiles.
- If you want to preview tags without pushing anything, use:

```bash
./scripts/tag-release.sh repo-reaper v0.1.0 --dry-run
```
