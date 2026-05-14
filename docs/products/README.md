# PatchHive Product Documentation

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

These pages are written as GitHub-facing product docs. They explain what each
PatchHive product does, where its boundaries are, how to run it, and how it fits
into the wider suite.

Every product remains independently runnable. HiveCore brings products together
through product-owned APIs; it does not replace standalone product operation.

## Product Index

| Product | Role | Docs |
| --- | --- | --- |
| RepoReaper | Autonomous patch and PR execution | [repo-reaper.md](repo-reaper.md) |
| SignalHive | Maintenance signal discovery | [signal-hive.md](signal-hive.md) |
| ReviewBee | PR review checklist extraction | [review-bee.md](review-bee.md) |
| TrustGate | Diff risk review and safety gating | [trust-gate.md](trust-gate.md) |
| RepoMemory | Durable repo knowledge and prompt context | [repo-memory.md](repo-memory.md) |
| FailGuard capability | Cross-cutting failure lessons and future guardrails | [failguard.md](failguard.md) |
| MergeKeeper | Merge readiness decisions | [merge-keeper.md](merge-keeper.md) |
| FlakeSting | Flaky CI detection | [flake-sting.md](flake-sting.md) |
| DepTriage | Dependency update prioritization | [dep-triage.md](dep-triage.md) |
| VulnTriage | Security finding prioritization | [vuln-triage.md](vuln-triage.md) |
| RefactorScout | Conservative refactor discovery | [refactor-scout.md](refactor-scout.md) |
| HiveCore | Suite control plane | [hive-core.md](hive-core.md) |

## Shared Expectations

- Products should be useful on their own.
- Product APIs should converge toward the shared contract in
  [../product-api-contract-v1.md](../product-api-contract-v1.md).
- Discovery products should honor allowlist, denylist, and opt-out controls.
- Write-capable products should prefer no action over weak output.
- HiveCore should consume product-owned APIs rather than private databases.
- Product pages should list the same local ports and standalone mirror links as
  their product `README.md` files.
- Run `npm run check:suite-drift` after changing product docs, package
  versions, ports, theme keys, or standalone CI conventions.
