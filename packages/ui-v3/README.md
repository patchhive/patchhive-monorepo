# PatchHive UI v3

UI v3 is the canonical shared visual system for PatchHive specialist products.
Its visual reference is `unified-ui-revamp-main/`, the Lovable project. The
current architecture contract lives in
[`docs/specialist-ui-architecture.md`](../../docs/specialist-ui-architecture.md).

The monorepo package is the source of truth. Specialist products use local
workspace paths during development; standalone exports consume the published
`@patchhivehq/ui-v3` version prepared by the suite release workflow.

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
- Use the shared startup-check list for checks surfaces so derived labels, status
  treatment, empty state, and backend-provided evidence remain consistent.
- Render GitHub as verified only when `github_ready` or
  `github.token_verified` is true. A configured token that failed or could not
  complete identity verification is unverified, not ready.
- Specialist products use `products/<product>/frontend/`; versioned migration
  directories are not active product targets.
- HiveCore is not part of the specialist UI architecture.
