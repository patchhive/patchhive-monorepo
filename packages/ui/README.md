# @patchhive/ui

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
} from "@patchhive/ui";
```

## Publishing

`@patchhive/ui` is published from the PatchHive monorepo.

The current publish workflow targets GitHub Packages under the `@patchhive` scope.

That is convenient for PatchHive-owned repos, but it also means consumer installs need package-registry authentication.

If PatchHive products need frictionless public installs later, publish this package to npmjs as a public package instead of relying only on GitHub Packages.

## Monorepo Note

Inside the monorepo, products may temporarily depend on this package through a local workspace or `file:` path.

Standalone product repositories should depend on a real versioned package release.
