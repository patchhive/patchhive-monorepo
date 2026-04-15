# @patchhivehq/product-shell

`@patchhivehq/product-shell` is the shared frontend shell layer for PatchHive products.

It holds the cross-product browser behavior that should feel identical everywhere: API-key bootstrap, session handling, authenticated fetch helpers, and the common product frame that wraps individual product panels.

## What It Includes

- `useApiKeyAuth` for shared API-key bootstrap and session handling
- `createApiFetcher` for authenticated requests to product backends
- `ProductSessionGate` for the shared login and loading boundary
- `ProductAppFrame` for the common PatchHive app frame

## Example

```js
import {
  createApiFetcher,
  ProductAppFrame,
  ProductSessionGate,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
```

## Publishing Model

`@patchhivehq/product-shell` is published to the public npm registry so standalone PatchHive product repositories can consume it as a normal versioned dependency.

The monorepo remains the source of truth for development and releases. The standalone `patchhive/product-shell` repository is a mirror for visibility, package-level CI, and external use.
