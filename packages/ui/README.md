# @patchhivehq/ui

Shared React UI primitives and components for PatchHive products.

This package is the reusable frontend layer behind PatchHive products such as RepoReaper.

## What It Includes

- theme helpers such as `applyTheme`
- shared style primitives such as `Btn`, `Input`, `Sel`, and `Divider`
- shared product components such as `AgentCard`, `DiffViewer`, `IssueRow`, and `LoginPage`

## Usage

```js
import {
  applyTheme,
  S,
  Btn,
  Input,
  Sel,
  PatchHiveHeader,
  PatchHiveFooter,
  TabBar,
  AgentCard,
  DiffViewer,
  LoginPage,
} from "@patchhivehq/ui";
```

## Publishing

`@patchhivehq/ui` is published from the PatchHive monorepo, with `patchhive/patchhive-ui` kept as the standalone package repo mirror.

The intended release target is the public npm registry, so standalone PatchHive product repositories and outside contributors can install it without package-registry authentication.

This package should be published as a public scoped package.

## Monorepo Note

The monorepo remains the source of truth for changes and releases.

Inside the monorepo, products may temporarily depend on this package through a local workspace or `file:` path.

Standalone product repositories should depend on a real versioned package release.
