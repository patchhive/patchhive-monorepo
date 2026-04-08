# UI Release Workflow

`@patchhivehq/ui` is versioned and released from the PatchHive monorepo.

The standalone `patchhive/patchhive-ui` repository is a mirror with package-focused history, not the source of truth.

## Normal Release Flow

1. Bump the package version in the monorepo:

```bash
./scripts/version-ui.sh patch
```

You can also choose `minor`, `major`, or an explicit version such as:

```bash
./scripts/version-ui.sh 0.2.0
```

If you want to preview the next version without editing files:

```bash
./scripts/version-ui.sh patch --dry-run
```

2. Review the changed files:

- `packages/ui/package.json`
- `package-lock.json`
- any workspace `package.json` files that depend on `@patchhivehq/ui`

3. Commit and push the monorepo changes.

4. Run the GitHub Actions workflow in `patchhive/patchhive2`:

- `Publish UI Package`

5. After the npm publish succeeds, update the standalone package mirror with a clean sync commit:

```bash
./scripts/sync-package-mirror.sh ui patchhive-ui main
```

6. Tag the release:

```bash
./scripts/tag-release.sh ui v0.1.0
```

## One-Time History Reset

If the standalone package mirror has old subtree-style history and you want to reset it to clean package-only sync commits, run:

```bash
./scripts/sync-package-mirror.sh ui patchhive-ui main --reset-history
```

That force-pushes a fresh root commit into `patchhive/patchhive-ui`.

## Principles

- The monorepo is always the source of truth.
- npm releases come from the monorepo.
- The standalone package repo is a visibility and issue-tracking mirror.
- Package mirror history should stay package-specific and easy to read.
