# Product Shell Release Workflow

`@patchhivehq/product-shell` is versioned and released from the PatchHive monorepo.

The standalone `patchhive/product-shell` repository is a mirror with package-focused history, not the source of truth.

## Normal Release Flow

1. Bump the package version in the monorepo:

```bash
./scripts/version-product-shell.sh patch
```

You can also choose `minor`, `major`, or an explicit version such as:

```bash
./scripts/version-product-shell.sh 0.2.0
```

If you want to preview the next version without editing files:

```bash
./scripts/version-product-shell.sh patch --dry-run
```

2. Review the changed files:

- `packages/product-shell/package.json`
- `package-lock.json`
- any workspace `package.json` files that depend on `@patchhivehq/product-shell`

3. Commit and push the monorepo changes.

4. Run the GitHub Actions workflow in `patchhive/patchhive2`:

- `Publish Product Shell Package`

5. After the npm publish succeeds, update the standalone package mirror with a clean sync commit:

```bash
./scripts/sync-package-mirror.sh product-shell product-shell main
```

6. Tag the release:

```bash
./scripts/tag-release.sh product-shell v0.1.0
```

## One-Time History Reset

If the standalone package mirror has old subtree-style history and you want to reset it to clean package-only sync commits, run:

```bash
./scripts/sync-package-mirror.sh product-shell product-shell main --reset-history
```

That force-pushes a fresh root commit into `patchhive/product-shell`.

## Principles

- The monorepo is always the source of truth.
- npm releases come from the monorepo.
- The standalone package repo is a visibility and issue-tracking mirror.
- Package mirror history should stay package-specific and easy to read.
