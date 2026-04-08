# @patchhivehq/product-shell

Shared frontend shell helpers for PatchHive products.

This package is the reusable frontend shell layer behind PatchHive products such as RepoReaper and SignalHive.

The standalone package repository is mirrored to `patchhive/product-shell`, but the PatchHive monorepo remains the source of truth for development and releases.

## What It Includes

- `useApiKeyAuth` for API-key-based product login bootstrap
- `createApiFetcher` for authenticated `fetch` calls against product backends

## Usage

```js
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell";
```

## Publishing

`@patchhivehq/product-shell` is published from the PatchHive monorepo so standalone product repos can consume it without local workspace paths.

The intended release target is the public npm registry, so standalone PatchHive product repositories and outside contributors can install it without package-registry authentication.

This package should be published as a public scoped package.

## Monorepo Note

The monorepo remains the source of truth for changes and releases.

Inside the monorepo, products may temporarily resolve this package through the npm workspace.

Standalone product repositories should depend on a real versioned package release.
