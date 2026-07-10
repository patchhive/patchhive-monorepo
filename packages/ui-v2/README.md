# PatchHive UI v2

> UI v2 remains available for existing radar/atlas product frontends. New
> specialist-product visual work uses `packages/ui-v3`, derived directly from
> `unified-ui-revamp-main`. Keep v2 stable during product-by-product migration.

This folder is the staging area for the next suite-wide PatchHive interface.
It is intentionally separate from `packages/ui` while the direction is still
being tuned across products.

## Current Direction

UI v2 is based on the SignalHive radar/atlas prototype:

- dense operator-console layout
- dark technical grid and scanline texture
- compact telemetry header
- bordered panels with hard edges and corner brackets
- product-colored signal accents
- explainable queues, metrics, and evidence trails
- interactive scope/radar surfaces where they clarify product state

SignalHive is the first product prototype and now has a React/Vite v2 app:

- `products/signal-hive/frontend-v2/index.html`
- `products/signal-hive/frontend-v2/src/`
- `products/signal-hive/frontend-v2/prototype-static.html` keeps the original
  static concept reference

## Product Migration Rule

Every product should migrate into the same v2 visual system before the old UI is
removed or marked legacy. Product-specific panels can differ, but the shell,
navigation, metric language, controls, cards, tabs, status chips, and empty/error
states should feel like one suite.

HiveCore is not exempt from the final v2 direction. Its current UI can stay
different while v2 is being proven, but HiveCore should eventually adopt the same
suite shell and visual language.

## Old UI Handling

Current production frontends remain in `products/<product>/frontend` until v2 is
ready. During the transition:

- v2 experiments live in `products/<product>/frontend-v2`
- shared v2 primitives and tokens live here
- old UI code is not renamed, deleted, or mixed with v2 work
- once every product is moved to v2, old UI code should be either moved under a
  clearly named legacy path or removed outright

## Promotion Path

1. Keep iterating on v2 prototypes until the suite language feels settled.
2. Extract stable tokens and reusable structure into shared React components.
3. Move those components into the published `@patchhivehq/ui` package, or turn
   this folder into the package source once the migration plan is final.
4. Port SignalHive first.
5. Port the rest of the products, including HiveCore.
6. Legacy or remove old UI code after all products are on v2.
