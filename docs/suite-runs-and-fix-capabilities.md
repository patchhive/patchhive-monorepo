# Suite Runs And Fix Capabilities

PatchHive's long-term system shape is not just a set of dashboards. HiveCore should eventually coordinate the whole suite: scan broadly, combine product evidence, identify eligible fixes, and dispatch approved product-owned repair actions.

The core model is:

- every product can scan
- some products can fix
- every fix is explicit, auditable, and routed through shared suite rules

This lets PatchHive become one coherent maintenance system without flattening the specialist products into one generic agent.

## Suite Run

A suite run is a HiveCore-owned orchestration run that asks enabled products to perform their product-owned scans against a shared target, schedule, or discovery scope.

Example flow:

1. HiveCore creates a suite run.
2. Enabled products scan in parallel or in dependency order.
3. Each product saves its own run detail and returns normalized run summaries.
4. The suite backend builds a single findings queue from product outputs.
5. Findings advertise their source product, target repo, severity, confidence, evidence, and fix eligibility.
6. HiveCore shows the operator what was found and what can be done.
7. The operator can approve individual fixes, a filtered fix batch, or a policy-constrained automated fix lane.
8. Product-owned fix actions run in isolated clones or workspaces.
9. TrustGate, RepoMemory, MergeKeeper, FlakeSting, and ReleaseSentry provide validation where relevant.
10. PatchHive opens clearly attributed pull requests only after the product's quality gates pass.

Suite runs should not require products to share private databases. Products keep their own evidence and expose enough normalized metadata for HiveCore to coordinate.

## Scan Versus Fix

Scanning should remain read-only by default.

Fixing should be a separate action with its own capability metadata, approval requirements, scopes, and run history.

That separation matters because it keeps low-friction discovery safe while still letting PatchHive become action-capable when confidence is high.

Recommended vocabulary:

- `scan`
  Read-only discovery or analysis.
- `finding`
  A normalized item discovered during a scan.
- `fix`
  A product-owned mutating action that attempts to resolve one finding or a bounded group of findings.
- `validation`
  Post-fix evidence such as tests, TrustGate review, merge readiness, CI health, or release readiness.
- `handoff`
  A product explicitly passing context to another product before action.

Normal product screens should keep scan and fix controls visually distinct. A button that says "Scan repo" should never mutate code. A button that says "Create refactor PR", "Apply dependency update", or "Fix eligible finding" can mutate, but only after the approval and guardrail path is visible.

## Suite Schedules

Scheduling is part of the suite-run model.

The v1 product surfaces proved that schedules matter, but v2 should avoid
duplicating schedule builders in every product. HiveCore should provide the
schedule board and `patchhive-backend` should own schedule storage, due-claiming,
dispatch, and audit events.

A suite schedule is a saved trigger for a product-owned action:

- read-only scan schedules can run automatically when policy allows
- mutating fix schedules should normally queue approval before dispatch
- each scheduled execution creates a normal product run and a suite event
- schedule history should link to the exact product run detail
- missed, skipped, blocked, or failed scheduled executions should be visible

Products should advertise which actions are scheduleable through capability
metadata. Scheduleability does not bypass product safety boundaries; it only
means the suite backend can trigger that action on a cadence.

Recommended schedule action metadata:

```json
{
  "id": "run_scan",
  "label": "Run scan",
  "scheduleable": true,
  "read_only": true,
  "mutates_repo": false,
  "requires_approval": false,
  "default_cadences": ["daily", "weekly"],
  "required_scopes": ["github:metadata:read"]
}
```

Recommended execution rules:

- Manual runs and scheduled runs share the same run ID format.
- Scheduled runs should include `trigger: "schedule"` and `schedule_id`.
- A disabled product, unhealthy product, missing credential, denylist hit, or
  approval requirement should create a skipped/blocked event, not a silent no-op.
- HiveCore should let the operator pause all schedules, pause a product's
  schedules, or pause one schedule.
- Product-local schedule endpoints may remain while gateway mode exists, but new
  v2 schedule UX should use suite schedules.

## Product Roles

Products should keep their specialist identity inside the suite run.

Suggested long-term ownership:

| Product | Suite role | Fix posture |
| --- | --- | --- |
| SignalHive | Discover maintenance pressure and candidate repos | Usually no direct fixes; hands candidates to specialist products |
| ReviewBee | Turn review feedback into actionable follow-up work | Can eventually apply selected review follow-ups or hand them to RepoReaper |
| TrustGate | Gate risky diffs and enforce policy | Should not directly fix; it approves, warns, or blocks fixes |
| RepoMemory | Provide durable conventions and failure lessons | Should not directly fix; it supplies context and policy memory |
| RepoReaper | Generate bug-fix patches and PRs | Primary autonomous code-fix product |
| MergeKeeper | Decide and repair merge readiness | Can eventually rebase/update PRs where permissions allow |
| FlakeSting | Detect flaky CI and test instability | Can eventually quarantine or repair flaky tests when confidence is high |
| DepTriage | Rank dependency update work | Can eventually open dependency update PRs |
| VulnTriage | Rank security findings | Can eventually fix dependency vulns or hand code vulns to RepoReaper |
| RefactorScout | Find safe cleanup opportunities | Can eventually create refactor PRs from selected leads |
| ReleaseSentry | Decide release readiness | Usually no direct fixes; can trigger release checklist remediation or handoffs |
| HiveCore | Control plane and suite-run orchestrator | Coordinates dispatch; should not bypass product-owned rules |

The rule is not "every product must mutate code." The rule is: if a product naturally owns a fix type, it should eventually advertise a fix action for that type.

## Fix Eligibility

Every finding in a suite queue should be able to describe whether it can be fixed.

Minimum fields:

```json
{
  "finding_id": "finding_01",
  "source_product": "refactor-scout",
  "target": {
    "repo": "owner/repo",
    "branch": "main"
  },
  "title": "Extract helper from long function",
  "summary": "Function spans 132 lines.",
  "confidence": 0.82,
  "severity": "medium",
  "fix_eligible": true,
  "fix_owner_product": "refactor-scout",
  "fix_action_id": "create_refactor_pr",
  "requires_approval": true,
  "required_scopes": ["github:contents:write", "github:pull_requests:write"],
  "evidence_paths": ["/api/products/refactor-scout/runs/run_123"]
}
```

Products can return richer product-specific data in `raw`, but HiveCore should not need product-specific translation to know whether a finding is actionable.

## Fix Action Contract

Fix actions should build on the existing `/capabilities` action dispatch contract.

Recommended action metadata:

```json
{
  "id": "create_refactor_pr",
  "label": "Create refactor PR",
  "method": "POST",
  "path": "/fix/refactor-pr",
  "description": "Create a small refactor PR for a selected RefactorScout lead.",
  "starts_run": true,
  "destructive": true,
  "mutates_repo": true,
  "opens_pr": true,
  "requires_approval": true,
  "required_scopes": [
    "github:contents:write",
    "github:pull_requests:write"
  ],
  "quality_gates": [
    "tests",
    "trust-gate",
    "repo-memory-context"
  ]
}
```

Fix actions should return a normal product run. The product owns the detailed evidence and state transitions, while HiveCore records the suite-level dispatch, approval, and result.

## Guardrails

Suite-level fixing must respect PatchHive's reputation and trust rules.

Required rules:

- All outbound PRs come from the PatchHive identity.
- PR bodies disclose the relevant PatchHive product and automation path.
- Products must honor allowlist, denylist, and opt-out policy before cloning, patching, or opening PRs.
- Fixes run in isolated clones or workspaces.
- Fixes should be small and product-specific.
- TrustGate should review generated diffs before PR creation when the fix mutates code.
- RepoMemory should provide conventions and FailGuard policy when available.
- MergeKeeper, FlakeSting, and ReleaseSentry should validate readiness where relevant.
- HiveCore should enforce suite-wide PR caps, confidence thresholds, and approval policy.
- Products should prefer no PR over weak output.

## Runtime Shape

During gateway mode, a suite run can call product backends through `patchhive-backend` proxy routes. Later, when product engines move in-process, the same suite-run contract should still hold.

The long-term backend should own:

- suite run records
- suite finding queue
- approval records
- dispatch logs
- product availability and health checks
- shared credentials and scopes
- suite schedules and schedule execution events
- suite-wide rate limits and PR caps

Products should own:

- product-specific scan logic
- product-specific fix logic
- evidence detail
- product run history
- validation specific to their domain

## Open Design Questions

- Should suite findings live only in suite tables, or should products also store normalized finding rows?
- Should HiveCore allow "fix all eligible findings" on day one, or only selected single-finding fixes first?
- How should batch approval policies be represented for trusted repos versus newly discovered repos?
- Which products should get write-capable fix actions first after RepoReaper?
- Should fix actions always open PRs, or can some create issues, comments, schedules, or configuration patches instead?

The direction is clear: PatchHive should scan like a suite, reason like a system, and fix through explicit product-owned actions.
