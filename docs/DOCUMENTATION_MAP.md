# PatchHive Documentation Map

The central index for everything under `docs/`. Start here when you don't know which file
holds what you need.

PatchHive is monorepo-first. Product work starts in `products/<slug>/` and is documented in
depth at `docs/products/<slug>.md`. Each product README is a short getting-started companion
that links back to its detailed doc (see [PRODUCT_README_TEMPLATE.md](PRODUCT_README_TEMPLATE.md)).

## How to navigate

- **"I want to run a product locally"** → the product's `README.md` (`## Quick Start`), then
  `docs/products/<slug>.md` for full config.
- **"I want the API for a product"** → `docs/products/<slug>.md` → `#api-endpoints`.
- **"I want to understand the whole suite / HiveCore"** → start with the Architecture notes
  and Platform docs below.
- **"I want the config conventions"** → [CONFIGURATION_STANDARDS.md](CONFIGURATION_STANDARDS.md).
- **"I want to write or audit a product doc"** → [PRODUCT_README_TEMPLATE.md](PRODUCT_README_TEMPLATE.md)
  (README standard) and the detailed-doc template referenced inside it.

## Product Docs

| Product | README | Detailed doc |
| --- | --- | --- |
| RepoReaper | [products/repo-reaper/README.md](../products/repo-reaper/README.md) | [products/repo-reaper.md](products/repo-reaper.md) |
| SignalHive | [products/signal-hive/README.md](../products/signal-hive/README.md) | [products/signal-hive.md](products/signal-hive.md) |
| ReviewBee | [products/review-bee/README.md](../products/review-bee/README.md) | [products/review-bee.md](products/review-bee.md) |
| TrustGate | [products/trust-gate/README.md](../products/trust-gate/README.md) | [products/trust-gate.md](products/trust-gate.md) |
| RepoMemory | [products/repo-memory/README.md](../products/repo-memory/README.md) | [products/repo-memory.md](products/repo-memory.md) |
| FailGuard capability | — | [products/failguard.md](products/failguard.md) |
| MergeKeeper | [products/merge-keeper/README.md](../products/merge-keeper/README.md) | [products/merge-keeper.md](products/merge-keeper.md) |
| FlakeSting | [products/flake-sting/README.md](../products/flake-sting/README.md) | [products/flake-sting.md](products/flake-sting.md) |
| DepTriage | [products/dep-triage/README.md](../products/dep-triage/README.md) | [products/dep-triage.md](products/dep-triage.md) |
| VulnTriage | [products/vuln-triage/README.md](../products/vuln-triage/README.md) | [products/vuln-triage.md](products/vuln-triage.md) |
| RefactorScout | [products/refactor-scout/README.md](../products/refactor-scout/README.md) | [products/refactor-scout.md](products/refactor-scout.md) |
| ReleaseSentry | [products/release-sentry/README.md](../products/release-sentry/README.md) | [products/release-sentry.md](products/release-sentry.md) |
| HiveCore | [products/hive-core/README.md](../products/hive-core/README.md) | [products/hive-core.md](products/hive-core.md) |

## Architecture Notes

- [Product operating modes](product-operating-modes.md) — directed targets vs autonomous discovery, and how products expose both.
- [Future product opportunities](future-product-opportunities.md) — overlap analysis and boundaries for recovered product concepts.
- [Shared Squad architecture](shared-squad-architecture.md) — RepoReaper's agent team as the first shared squad implementation.

## Platform Docs

| Doc | What it covers |
| --- | --- |
| [Platform guardrails](platform-guardrails.md) | Reputation and alignment rules as the suite grows toward HiveCore. |
| [Suite stabilization plan](suite-stabilization-plan.md) | Current stabilization priorities and tracking. |
| [Suite backend direction](suite-backend-direction.md) | The shared-backend runtime direction (Option A) for the suite. |
| [Product API contract v1](product-api-contract-v1.md) | First shared contract target for product backends. |
| [GitHub token scopes](github-token-scopes.md) | Narrowest token scopes per product. |
| [SQLite connection strategy](sqlite-connection-strategy.md) | Shared `SqlitePool` usage across backends. |
| [PatchHive Registry](patchhive-registry.md) | Future hosted service for HiveCore to publish product catalogs. |
| [MaintainerBot operating mode](maintainerbot-operating-mode.md) | Packaged bot posture, not a standalone product. |
| [HiveCore first-stack readiness](hivecore-first-stack-readiness.md) | Readiness audit for the first HiveCore product stack. |
| [HiveCore suite bootstrap wizard](hivecore-suite-bootstrap-wizard.md) | Bootstrap wizard design. |
| [Suite runs and fix capabilities](suite-runs-and-fix-capabilities.md) | Long-term system shape: scan → combine evidence → fix. |
| [Public release readiness](public-release-readiness.md) | Criteria for going public. |

## Release & Workflow Docs

| Doc | What it covers |
| --- | --- |
| [Release checklist](release-checklist.md) | Monorepo-first release steps. |
| [Product export workflow](product-export-workflow.md) | How products are mirrored into standalone repos. |
| [Product starter workflow](product-starter-workflow.md) | Scaffolding a new product from the starter. |
| [UI release workflow](ui-release-workflow.md) | Releasing `@patchhivehq/ui`. |
| [Product shell release workflow](product-shell-release-workflow.md) | Releasing `@patchhivehq/product-shell`. |
| [UI v2 migration](ui-v2-migration.md) | Active/legacy v2 migration track. |
| [UI v3 migration](ui-v3-migration.md) | v3 visual-language unification. |

## Standards & Planning

| Doc | What it covers |
| --- | --- |
| [PRODUCT_README_TEMPLATE.md](PRODUCT_README_TEMPLATE.md) | **Canonical README structure + Safety Boundary standard.** Read before writing a product README. |
| [CONFIGURATION_STANDARDS.md](CONFIGURATION_STANDARDS.md) | De-facto config naming, suffixes, and known deviations. |
| [PatchHive Improvements](PatchHive_Improvements.md) | The master improvement analysis (10 findings + action plan). |
| [Future plans](FUTURE_PLANS.md) | Planning scratchpad. |

## See Also

- Top-level suite README: [../README.md](../README.md)
- Contributing: [../CONTRIBUTING.md](../CONTRIBUTING.md)
- Security: [../SECURITY.md](../SECURITY.md)
