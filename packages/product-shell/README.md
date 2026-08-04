# @patchhivehq/product-shell

`@patchhivehq/product-shell` is the shared browser auth and runtime layer for PatchHive products.

It holds cross-product API-key bootstrap, session handling, authenticated fetch
helpers, and compatibility frames. Canonical specialist visual structure lives
in `@patchhivehq/ui`.

## What It Includes

- `useApiKeyAuth` for shared API-key bootstrap and session handling
- `createApiFetcher` for authenticated requests to product backends
- compatibility `ProductSessionGate`, `ProductAppFrame`, and
  `ProductSetupWizard` components for compatibility consumers

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
