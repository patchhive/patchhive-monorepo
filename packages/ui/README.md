# @patchhivehq/ui

`@patchhivehq/ui` is the canonical shared React interface for PatchHive.

It contains the specialist product shell, theme, diagnostics, controls,
history, scheduling, and workspace primitives used by all eleven specialist
products. A small set of earlier primitives remains exported for the published
`@patchhivehq/product-shell` and `@patchhivehq/ai-models` APIs; those exports are
compatibility support, not a second visual system.

Import the package and its shared stylesheet:

```jsx
import { ProductHeader, ProductShell, Surface } from "@patchhivehq/ui";
import "@patchhivehq/ui/styles.css";
```

The package is published from the monorepo to the public npm registry so
standalone PatchHive product repositories can install it without private
registry authentication.
