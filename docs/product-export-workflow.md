# Product And Package Export Workflow

PatchHive uses the monorepo as the source of truth.

Standalone product repositories are exported from the monorepo when a product is ready for its own GitHub presence.

Shared packages can be exported the same way when you want them to have their own GitHub identity.

## Principles

- Develop products in the monorepo first.
- Develop shared packages in the monorepo first.
- Treat standalone product repositories as exported mirrors, not the primary development home.
- Treat standalone package repositories as exported mirrors, not the primary development home.
- Re-export products and packages from the monorepo instead of manually copying files around.

## Shared Packages

Product exports do not carry `packages/` with them.

That is intentional.

Standalone product repositories should:

- depend on published shared packages such as `@patchhivehq/ui`
- depend on published shared packages such as `@patchhivehq/product-shell`
- use shared service contracts for things like `PATCHHIVE_AI_URL`
- avoid local `file:` dependencies back into the monorepo

`@patchhivehq/ui` is intended to publish to the public npm registry so standalone products can install it without package-registry authentication.
`@patchhivehq/product-shell` follows the same pattern.

That means:

- standalone product repositories can depend on normal semver releases
- outside contributors can run `npm install` without GitHub package tokens
- PatchHive only needs npm publishing credentials during release, not during consumer installs

For example, RepoReaper's frontend currently uses a local dependency while it lives inside the monorepo. When RepoReaper becomes a standalone repository, that dependency should be changed from a local path to a published package version.

## Export Script

Use:

```bash
./scripts/export-product.sh <product-name>
```

Example:

```bash
./scripts/export-product.sh repo-reaper
```

This creates a local export branch from `products/repo-reaper`.

If you want to push directly to a standalone remote:

```bash
./scripts/export-product.sh repo-reaper repo-reaper main
```

That will:

1. create a subtree export branch
2. push that branch to the `repo-reaper` remote's `main` branch

The script is intentionally safe:

- it does not overwrite an existing export branch
- if `export/<product>` already exists, it creates a timestamped branch name instead

## Package Export Script

Use:

```bash
./scripts/export-package.sh <package-name>
```

Example:

```bash
./scripts/export-package.sh ui
```

If you want to push directly to a standalone package remote:

```bash
./scripts/export-package.sh ui patchhive-ui main
```

That creates a subtree export branch from `packages/ui` and can push it directly into a standalone package repository.

## Package Mirror Sync Script

For shared package repositories, PatchHive prefers clean package-only mirror history over raw subtree history.

After the first export, use:

```bash
./scripts/sync-package-mirror.sh ui patchhive-ui main
```

That creates one package-focused sync commit in the standalone mirror repository instead of replaying mixed monorepo commit messages.

If you want to reset an existing package mirror onto the clean sync history model, use:

```bash
./scripts/sync-package-mirror.sh ui patchhive-ui main --reset-history
```

That force-pushes a fresh root commit into the standalone package repository.

## Recommended First Export

1. Create an empty GitHub repository for the product.
2. Add it as a remote in the monorepo.
3. Run the export script.
4. Push the export branch to the product repo.
5. Update the standalone product repo to use published shared packages.

## Recommended Package Export

1. Create an empty GitHub repository for the package.
2. Add it as a remote in the monorepo.
3. Run the package export script.
4. Push the export branch to the package repo.
5. Keep releases and canonical history rooted in the monorepo.
6. After the initial export, prefer `sync-package-mirror.sh` for future mirror updates.

## Day-To-Day Workflow

The intended long-term flow is:

1. Build inside the monorepo.
2. Commit and push monorepo changes first.
3. Export a product or package when you want its standalone repository updated.
4. Push the export branch into the corresponding standalone repository.

This keeps one clean source of truth while still giving each product its own GitHub identity.
