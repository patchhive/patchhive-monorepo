# Tendwright Documentation Map

The central index for everything under `docs/`. Start here when you don't know which file
holds what you need.

Tendwright is monorepo-first. Product work starts in `products/<slug>/` and is documented in
depth at `docs/products/<slug>.md`. Each product README is a short getting-started companion
that links back to its detailed doc (see [PRODUCT_README_TEMPLATE.md](PRODUCT_README_TEMPLATE.md)).

When a Tendwright discussion, note, or external conversation reaches a concrete
architecture, product, contract, workflow, or safety decision, record it in the
canonical repository documentation before implementation or model handoff.
Brainstorming may remain in planning docs, but unresolved choices must be marked
open rather than written as settled architecture.

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
- [Product naming strategy](product-naming-strategy.md) — descriptive external names, internal apiary vocabulary, protected names, and rename safety.
- [Future product opportunities](future-product-opportunities.md) — overlap analysis and boundaries for recovered product concepts.
- [Email and agentic webmail architecture](inbound-email-architecture.md) — native mailbox access, agent-assisted triage and replies, product dispatch, approval, and escalation boundaries.
- [Shared Squad architecture](shared-squad-architecture.md) — RepoReaper's agent team as the first shared squad implementation.

## Platform Docs

| Doc | What it covers |
| --- | --- |
| [Platform guardrails](platform-guardrails.md) | Reputation and alignment rules as the suite grows toward HiveCore. |
| [ChatGPT subscription AI](chatgpt-subscription-ai.md) | Official Codex SDK/CLI subscription auth, shared gateway flow, typed status, and credential boundary. |
| [Suite stabilization plan](suite-stabilization-plan.md) | Current stabilization priorities and tracking. |
| [Suite backend direction](suite-backend-direction.md) | The shared-backend runtime direction (Option A) for the suite. |
| [Product API contract v1](product-api-contract-v1.md) | First shared contract target for product backends. |
| [GitHub token scopes](github-token-scopes.md) | Narrowest token scopes per product. |
| [SQLite connection strategy](sqlite-connection-strategy.md) | Shared `SqlitePool` usage across backends. |
| [PatchHive Registry](patchhive-registry.md) | Future hosted service for HiveCore to publish product catalogs. |
| [MaintainerBot operating mode](maintainerbot-operating-mode.md) | Packaged bot posture, not a standalone product. |
| [HiveCore architecture](hivecore-architecture.md) | **Canonical HiveCore design.** Four layers, current-implementation blockers, and the build order toward running the whole suite. |
| [Autonomous maintenance loop](autonomous-maintenance-loop.md) | Canonical suite-mode flow from mandate and discovery through exact-diff authorization, publication, reconciliation, and memory feedback. |
| [HiveCore first-stack readiness](hivecore-first-stack-readiness.md) | Readiness audit for the first HiveCore product stack. |
| [HiveCore suite bootstrap wizard](hivecore-suite-bootstrap-wizard.md) | Bootstrap wizard design. |
| [Suite runs and fix capabilities](suite-runs-and-fix-capabilities.md) | Long-term system shape: scan → combine evidence → fix. |
| [HiveCore repository safety and PR budgets](hivecore-repository-safety-and-pr-budgets.md) | Future public opt-out, trusted-repo policy, and hierarchical PR-budget design. |
| [Public release readiness](public-release-readiness.md) | Criteria for going public. |

## Release & Workflow Docs

| Doc | What it covers |
| --- | --- |
| [Release checklist](release-checklist.md) | Monorepo-first release steps. |
| [Product export workflow](product-export-workflow.md) | How products are mirrored into standalone repos. |
| [Product starter workflow](product-starter-workflow.md) | Scaffolding a new product from the starter. |
| [UI release workflow](ui-release-workflow.md) | Releasing the control-plane UI and specialist UI v3 packages. |
| [Product shell release workflow](product-shell-release-workflow.md) | Releasing `@patchhivehq/product-shell`. |
| [Specialist UI architecture](specialist-ui-architecture.md) | Canonical specialist frontend structure, shared UI contracts, and verification rules. |

## Standards & Planning

| Doc | What it covers |
| --- | --- |
| [PRODUCT_README_TEMPLATE.md](PRODUCT_README_TEMPLATE.md) | **Canonical README structure + Safety Boundary standard.** Read before writing a product README. |
| [CONFIGURATION_STANDARDS.md](CONFIGURATION_STANDARDS.md) | De-facto config naming, suffixes, and known deviations. |
| [Product naming strategy](product-naming-strategy.md) | Customer-facing naming rules, internal apiary vocabulary, and rename candidates. |
| [Future plans](FUTURE_PLANS.md) | Planning scratchpad. |
| [Full review — 2026-07-31](code-review-2026-07-31.md) | Reproduced findings across code, docs, config and the safety model, with severity and suggested order. |
| [Security, performance, and safety review — 2026-08-03](security-performance-safety-review-2026-08-03.md) | Post-UI-consolidation runtime, dependency, export/release, and autonomous-safety review with repaired findings and remaining deployment gates. |

## See Also

- Top-level suite README: [../README.md](../README.md)
- Contributing: [../CONTRIBUTING.md](../CONTRIBUTING.md)
- Security: [../SECURITY.md](../SECURITY.md)
