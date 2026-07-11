# UI v3 Migration

PatchHive UI v3 gives every specialist product the exact visual language of the
Lovable project in `unified-ui-revamp-main/` while keeping each product a
separate application.

## Source of Truth

The Lovable project is executable design source, not loose inspiration. UI v3
must preserve its typography, spacing, geometry, glass surfaces, shadows,
background treatments, controls, light/dark palettes, motion, and responsive
behavior. Do not redraw the interface from screenshots and do not use
`prototypes/vuln-triage-calm-mockup.html` as a replacement design.

The Lovable project currently includes prototype data and a VulnTriage-specific
route. Production integration therefore extracts its visual implementation but
replaces the prototype store with each product's real auth, runtime, API, and
workflow state.

## Structure

- `packages/ui-v3/` owns shared Lovable-derived tokens, theme behavior, shell,
  header, surfaces, metric cards, progressive lists, dashboard filter/sort and
  saved-view controls, and filterable activity timelines.
- `products/<product>/frontend-v3/` owns an in-progress product migration;
  after parity and promotion, that implementation moves to the canonical
  `products/<product>/frontend/` path.
- Existing v2 frontends remain available only until product-specific parity is
  verified and the launch/Docker default is intentionally promoted.
- HiveCore remains on its separate control-plane interface.

## Product Variation

The geometry and component language stay consistent. Products may vary:

- product name, icon, subtitle, and accent palette
- navigation labels and available sections
- metrics, evidence types, queues, forms, and actions
- API data mapping and product-owned workflow behavior

Product variation must not silently introduce a different card system, shell,
typography scale, spacing system, or theme implementation.

## Theme Contract

- Store the explicit preference in localStorage as `patchhive.theme`.
- Supported stored values are `light` and `dark`.
- With no stored value, follow `prefers-color-scheme`.
- Synchronize theme changes across open browser tabs.
- Apply the root `.dark` class before React starts to avoid a theme flash.
- The preference is shared across all specialist products.

## Shared Workspace Contract

- Product queues should use the shared progressive list when a long result set
  would overwhelm the dashboard; the default collapsed count is six.
- Filter/sort state and named dashboard views use the shared v3 saved-view hook
  and remain local to the browser unless a future backend contract replaces it.
- Activity timelines use the shared type filters and jump controls. Products
  still own the event types and the mapping from product evidence to events.
- Scan warning surfaces, GitHub permission guidance, and Markdown-copy feedback
  are shared v3 controls; products supply warning interpretation, required
  scopes, and report content.
- Specialist footers preserve the v2 identity wording: `<Product> by
  PatchHive`, the product subtitle, and `Autonomous maintenance suite`.

## Migration Order

1. Port every product already mounted in-process by the unified backend.
2. Extract any newly proven shared controls into `@patchhivehq/ui-v3`.
3. Continue unified-backend engine migration only after the integrated set has
   stable v3 frontends.
4. Audit product behavior, safety controls, Docker paths, and responsive layout.
5. Promote v3 product by product; legacy or remove v2 only after parity.

## Current Status

| Product | v3 folder | Status |
| --- | --- | --- |
| MergeKeeper | `products/merge-keeper/frontend-v3` | v1/v2 parity implemented and locally verified; awaiting live ready/blocked/publish acceptance and operator sign-off |
| ReleaseSentry | `products/release-sentry/frontend-v3` | Unified-backend v3 workspace implemented |
| DepTriage | `products/dep-triage/frontend-v3` | Unified-backend v3 workspace implemented |
| VulnTriage | `products/vuln-triage/frontend` | Promoted after final parity audit; v1/v2 removed |
| FlakeSting | `products/flake-sting/frontend-v3` | Unified-backend v3 workspace implemented |
| Other specialist products | not created | Wait until their product engine is integrated in-process |
| HiveCore | not applicable | Intentionally separate |

The integrated set is derived from both the registry manifests and executable
route mounts. As of 2026-07-10, these five products have
`migration_stage = "integrated"`, are initialized by
`services/patchhive-backend/src/products.rs`, and are nested directly by
`services/patchhive-backend/src/routes.rs`. Gateway-ready products are not
counted as complete for v3 migration ordering.
