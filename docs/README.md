# PatchHive Documentation

<p align="center">
  <img src="../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

This directory holds the operational and GitHub-facing documentation for the
PatchHive suite.

PatchHive is monorepo-first. Product work starts here, then product directories
are exported into standalone repositories under the `patchhive` GitHub
organization when they are ready to stand alone.

## Product Docs

The product documentation set is in [products/](products/).

| Product | Documentation |
| --- | --- |
| RepoReaper | [products/repo-reaper.md](products/repo-reaper.md) |
| SignalHive | [products/signal-hive.md](products/signal-hive.md) |
| ReviewBee | [products/review-bee.md](products/review-bee.md) |
| TrustGate | [products/trust-gate.md](products/trust-gate.md) |
| RepoMemory | [products/repo-memory.md](products/repo-memory.md) |
| FailGuard capability | [products/failguard.md](products/failguard.md) |
| MergeKeeper | [products/merge-keeper.md](products/merge-keeper.md) |
| FlakeSting | [products/flake-sting.md](products/flake-sting.md) |
| DepTriage | [products/dep-triage.md](products/dep-triage.md) |
| VulnTriage | [products/vuln-triage.md](products/vuln-triage.md) |
| RefactorScout | [products/refactor-scout.md](products/refactor-scout.md) |
| ReleaseSentry | [products/release-sentry.md](products/release-sentry.md) |
| HiveCore | [products/hive-core.md](products/hive-core.md) |

## Platform Docs

- [Platform guardrails](platform-guardrails.md)
- [HiveCore first-stack readiness audit](hivecore-first-stack-readiness.md)
- [HiveCore suite bootstrap wizard](hivecore-suite-bootstrap-wizard.md)
- [Suite backend direction](suite-backend-direction.md)
- [Product API contract v1](product-api-contract-v1.md)
- [MaintainerBot operating mode](maintainerbot-operating-mode.md)
- [SQLite connection strategy](sqlite-connection-strategy.md)
- [Product export workflow](product-export-workflow.md)
- [Product starter workflow](product-starter-workflow.md)
- [Release checklist](release-checklist.md)
- [UI release workflow](ui-release-workflow.md)
- [UI v2 migration](ui-v2-migration.md)
- [Product shell release workflow](product-shell-release-workflow.md)
- Suite drift guard: `npm run check:suite-drift`
