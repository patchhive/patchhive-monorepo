# UI v2 Migration

PatchHive is moving toward a shared v2 frontend language. The current candidate
direction is the SignalHive radar/atlas React prototype in
`products/signal-hive/frontend-v2/`. The original static concept is preserved as
`products/signal-hive/frontend-v2/prototype-static.html`.

## Why

The product suite should feel like one coherent agent instead of separate apps
that happen to share a logo. RepoReaper, SignalHive, TrustGate, RepoMemory,
MergeKeeper, FlakeSting, DepTriage, VulnTriage, RefactorScout, ReleaseSentry,
and HiveCore should all carry the same shell, visual density, control language,
and evidence-first presentation.

## Working Structure

- `packages/ui-v2/` holds the staging notes and shared v2 design tokens while
  the design is still moving.
- `products/<product>/frontend-v2/` is the place for product-specific v2
  prototypes.
- `products/<product>/frontend/` remains the current production frontend until
  that product is intentionally ported.

## Old UI Code

Do not mix v2 work into the old UI code while the direction is still being
settled. The old UI should remain available for normal product development and
local runs.

After every product has moved to v2, choose one cleanup path:

- move the old UI code into a clearly named legacy location if it still has
  reference value
- remove it entirely if v2 has fully replaced it

## HiveCore

HiveCore does not currently follow this UI direction. That is acceptable during
the v2 design/prototype phase, but it is not the target end state. HiveCore
should eventually use the same v2 suite shell and design language while keeping
its control-plane-specific panels.

## Product Status

| Product | v2 folder | Status |
| --- | --- | --- |
| SignalHive | `products/signal-hive/frontend-v2` | React prototype active |
| RepoReaper | pending | Not started |
| ReviewBee | pending | Not started |
| TrustGate | `products/trust-gate/frontend-v2` | React prototype active |
| RepoMemory | `products/repo-memory/frontend-v2` | React prototype active |
| MergeKeeper | pending | Not started |
| FlakeSting | pending | Not started |
| DepTriage | pending | Not started |
| VulnTriage | pending | Not started |
| RefactorScout | pending | Not started |
| ReleaseSentry | pending | Not started |
| HiveCore | pending | Must converge before final cleanup |

## Promotion Checklist

1. Settle the shared v2 visual language with SignalHive.
2. Identify reusable shell pieces: header, nav, sidebar, metric band, panels,
   chips, buttons, forms, queue cards, diagnostics, empty states, and errors.
3. Extract reusable React components into the shared UI package path.
4. Continue extracting SignalHive v2 pieces into reusable components.
5. Port at least one non-SignalHive product to validate the system.
6. Port HiveCore and the remaining products.
7. Legacy or remove the old UI code.
