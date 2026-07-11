# PatchHive UI v3

UI v3 is the shared visual system for PatchHive specialist products. Its
canonical visual source is `unified-ui-revamp-main/`, the Lovable project.

Rules:

- Preserve the Lovable typography, spacing, radii, glass surfaces, shadows,
  backgrounds, responsive behavior, and light/dark presentation exactly.
- Reuse these components and tokens. Do not redraw the UI from screenshots or
  replace it with a static mockup.
- Each product remains a separate frontend and supplies its own copy, data,
  routes, controls, and product accent.
- `patchhive.theme` is suite-wide and persists `light` or `dark` in localStorage.
- Use the shared history dashboard for saved-run surfaces so refresh, counts,
  search, filters, sorting, saved views, empty state, and progressive disclosure
  remain consistent while products supply their own filters and row evidence.
- HiveCore is not part of the specialist-product v3 migration.
