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
  that product is intentionally ported. After a parity audit, move the old UI to
  `products/<product>/frontend-legacy/` and make the v2 frontend the active
  frontend for local and Docker runs.

## Old UI Code

Do not mix v2 work into the old UI code while the direction is still being
settled. Before a product passes parity, the old UI should remain available for
normal product development and local runs. After parity, keep it only as
`frontend-legacy/` reference material until deletion.

During gateway testing, a v2 product may carry a lightweight replacement for an
old configuration surface when that is enough to test the live workflow. Any
missing old-UI behavior must be documented in the product docs before the old UI
is retired. RepoReaper is the current example: its v2 shell can recruit a
starter agent team and gate runs honestly, but full team presets, richer
provider/model controls, and the HiveCore setup path remain deferred until the
RepoReaper unified-backend pass. Its active team and preset secret fields are
already encrypted at rest when a RepoReaper or suite encryption key is
configured.

## Required V1 Parity

The old product UIs included safety and scope controls that cannot disappear in
v2. Before old UI code is moved to legacy or removed, v2 must cover the
operator workflows that keep autonomous discovery and outbound contribution
bounded.

Required v2 controls:

- repo allowlist management
- repo denylist/blocklist management
- durable opt-out management
- saved target scopes for discovery runs
- schedule visibility and pause/resume controls
- per-run policy readouts showing which allowlist, denylist, opt-out, schedule,
  or saved scope shaped the run
- clear empty states when a product cannot run because no target, saved scope,
  or allowlist is configured

These controls should become suite-level surfaces in HiveCore and
`patchhive-backend`, not cloned one-off tables in every product. During gateway
mode, product-local endpoints can remain as adapters, but v2 UI should make the
policy posture visible wherever a product can discover repos, clone repos, open
PRs, publish comments, or dispatch scheduled work.

Minimum UI expectations:

- Directed forms should show the concrete target and the active policy status.
- Autonomous forms should show the discovery scope, repo caps, and active
  allow/deny/opt-out counts before the run starts.
- History rows should identify whether a run was directed, targeted, scheduled,
  or autonomous.
- Write-capable products should surface PR caps, confidence thresholds, and
  approval requirements beside the run action.
- Opt-out should always win visually and behaviorally.

## V1 Audit Log

Each product needs at least one old-UI parity pass before its old frontend is
legacy or removed. Record the audited files, workflows that v2 covers, any
intentional v2 changes, and deferred polish in the product docs.

- **MergeKeeper** audited on 2026-07-03. See
  `docs/products/merge-keeper.md#ui-v1-to-v2-parity-audit`. V2 covers directed
  PR assessment, publish toggles, readiness evidence, history loading, health
  checks, integrations, and GitHub artifact links/copy actions. Deferred polish:
  compact reviewer-state strip and optional inline report preview if live use
  proves they are worth the extra density.
- **ReleaseSentry** audited on 2026-07-03. See
  `docs/products/release-sentry.md#ui-v1-to-v2-parity-audit`. V2 covers release
  intake, ready/watch/hold decisions, release evidence, history loading,
  selected-run detail, health checks, GitHub readiness, and unified-backend
  routing. Deferred polish: compact evidence/link expansion for release,
  workflow, changelog, and repository artifacts before old UI deletion if live
  testing shows operators need the direct jumps.
- **DepTriage** audited on 2026-07-05. See
  `docs/products/dep-triage.md#ui-v1-to-v2-parity-audit`. V2 covers directed
  dependency scans, PR limits, optional Dependabot alert reads, update/watch/
  ignore queues, Markdown summary copy, queue sorting, manifest/reason evidence,
  PR and alert links, history loading, health checks, startup checks, and
  unified-backend routing. Deferred polish: suite-owned scope controls and any
  future approval-gated dependency execution surface.

After every product has moved to v2, choose one cleanup path:

- move the old UI code into a clearly named legacy location if it still has
  reference value
- remove it entirely if v2 has fully replaced it

## HiveCore

HiveCore now has a v2 demo after the other 11 product surfaces proved the shared
suite shell and design language. Its v2 surface should stay control-plane-first:
product health, launcher state, contract drift, shared defaults, and
cross-product handoffs rather than another single-product radar board.

## Product Status

| Product | v2 folder | Status |
| --- | --- | --- |
| SignalHive | `products/signal-hive/frontend-v2` | React prototype active |
| RepoReaper | `products/repo-reaper/frontend-v2` | React prototype active |
| ReviewBee | `products/review-bee/frontend-v2` | React prototype active |
| TrustGate | `products/trust-gate/frontend-v2` | React prototype active |
| RepoMemory | `products/repo-memory/frontend-v2` | React prototype active |
| MergeKeeper | `products/merge-keeper/frontend-v2` | v2 active; v1 moved to `frontend-legacy` |
| FlakeSting | `products/flake-sting/frontend-v2` | React prototype active |
| DepTriage | `products/dep-triage/frontend-v2` | v2 active; v1 moved to `frontend-legacy` |
| VulnTriage | `products/vuln-triage/frontend-v2` | React prototype active |
| RefactorScout | `products/refactor-scout/frontend-v2` | React prototype active |
| ReleaseSentry | `products/release-sentry/frontend-v2` | v2 active; v1 moved to `frontend-legacy` |
| HiveCore | `products/hive-core/frontend-v2` | Control-plane demo active |

## Promotion Checklist

1. Settle the shared v2 visual language with SignalHive.
2. Identify reusable shell pieces: header, nav, sidebar, metric band, panels,
   chips, buttons, forms, queue cards, diagnostics, empty states, and errors.
3. Extract reusable React components into the shared UI package path.
4. Continue extracting SignalHive v2 pieces into reusable components.
5. Port at least one non-SignalHive product to validate the system.
6. Promote HiveCore's control-plane demo into the same production-port path as
   the specialist products.
7. Legacy or remove the old UI code.
