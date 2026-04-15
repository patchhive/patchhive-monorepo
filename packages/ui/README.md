# @patchhivehq/ui

`@patchhivehq/ui` is the shared React component library for PatchHive products.

It provides the common visual layer behind the PatchHive suite: themes, layout primitives, shared navigation, status components, and product-facing UI elements that let individual products feel related without looking copy-pasted.

## What It Includes

- theme helpers such as `applyTheme` and product theme maps
- layout primitives such as buttons, inputs, tabs, dividers, and empty states
- shared product chrome such as headers, footers, and status badges
- reusable product components such as `AgentCard`, `DiffViewer`, `IssueRow`, and `LoginPage`

## Example

```js
import {
  applyTheme,
  Btn,
  Input,
  PatchHiveHeader,
  PatchHiveFooter,
  TabBar,
  LoginPage,
} from "@patchhivehq/ui";
```

## Publishing Model

`@patchhivehq/ui` is published to the public npm registry so standalone PatchHive product repositories can install it without private package registry auth.

The monorepo is the source of truth for development and releases. The standalone `patchhive/patchhive-ui` repository is a mirror for visibility, package-level CI, and external consumption.
