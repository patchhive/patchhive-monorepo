# @patchhivehq/ui

`@patchhivehq/ui` is PatchHive's control-plane and compatibility React package.
Canonical specialist products use `@patchhivehq/ui-v3`.

It preserves the primitives still used by HiveCore, product-shell compatibility
components, and ai-models. Do not add new specialist product surfaces here.

## What It Includes

- theme helpers such as `applyTheme` and product theme maps
- layout primitives such as buttons, inputs, tabs, dividers, and empty states
- shared product chrome such as headers, footers, and status badges
- reusable product components such as `AgentCard`, `DiffViewer`, `IssueRow`, `LoginPage`, and `PanelErrorBoundary`

## Example

```js
import {
  applyTheme,
  Btn,
  Input,
  PatchHiveHeader,
  PatchHiveFooter,
  PanelErrorBoundary,
  TabBar,
  LoginPage,
} from "@patchhivehq/ui";
```

## Publishing Model

`@patchhivehq/ui` is published to the public npm registry so its remaining standalone consumers can install it without private package registry auth.

The monorepo is the source of truth for development and releases. The standalone `patchhive/patchhive-ui` repository is a mirror for visibility, package-level CI, and external consumption.
