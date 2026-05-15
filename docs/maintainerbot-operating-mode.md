# MaintainerBot Operating Mode

MaintainerBot is not planned as a separate PatchHive product. It is a packaged
operating mode, service offering, and public identity built on top of the
existing PatchHive suite.

The important distinction:

- PatchHive is the platform.
- HiveCore is the suite brain and command center.
- The specialist products do the actual maintenance work.
- MaintainerBot is the named workflow for keeping Rust crates healthy.

This keeps the architecture clean. PatchHive should not grow another isolated
product that competes with RepoReaper, SignalHive, DepTriage, VulnTriage,
RepoMemory, TrustGate, and HiveCore. Instead, MaintainerBot should coordinate
those products toward one clear customer outcome: maintained Rust crates.

## Why It Fits PatchHive

PatchHive's north star is autonomous, outbound maintenance contribution. The
suite should find work on its own, act under the PatchHive identity, and produce
reviewable maintenance output.

MaintainerBot is a concrete market-shaped expression of that north star. Instead
of selling "an AI maintenance dashboard," it sells a result:

> This Rust crate stays healthy, current, secure, and reviewable.

That is easier to explain, easier to buy, and easier to measure.

It also fits the trust model. MaintainerBot should not pretend to be a human
maintainer. It should contribute clearly as PatchHive, produce visible evidence,
and let maintainers inspect its history.

## Product Mapping

MaintainerBot should be implemented as a HiveCore workflow that coordinates the
existing products.

| Need | PatchHive capability |
| --- | --- |
| Crate health visibility | SignalHive |
| Dependency update triage | DepTriage |
| Security/advisory triage | VulnTriage |
| CI instability and flaky jobs | FlakeSting |
| Project conventions and past decisions | RepoMemory |
| Risk checks for proposed changes | TrustGate |
| Patch and PR generation | RepoReaper |
| PR readiness and merge blockers | MergeKeeper |
| Review-thread follow-up | ReviewBee |
| Durable lessons from bad outcomes | FailGuard |
| Workflow orchestration and SLA evidence | HiveCore |

MaintainerBot is the workflow that binds these pieces together for Rust crate
maintenance.

## What MaintainerBot Should Do First

The first version should stay conservative and review-first.

MaintainerBot v1 should:

- Monitor a configured Rust crate repository.
- Detect stale dependency pressure, security advisories, CI failures, stale PRs,
  issue backlog pressure, and documentation drift.
- Open reviewable pull requests for low-risk maintenance work.
- Run TrustGate before any write-capable action.
- Use RepoMemory to respect crate-specific conventions.
- Keep an evidence trail in HiveCore showing what was checked, what was changed,
  and what remains blocked.
- Report a clear crate health/SLA status.

MaintainerBot v1 should not:

- Publish to crates.io automatically.
- Merge pull requests automatically.
- Take ownership of maintainer communication without clear boundaries.
- Make broad behavioral rewrites without explicit maintainer approval.
- Hide that the work is autonomous.

Publishing and merging are high-trust operations. They can become later
capabilities, but only after the reviewable PR loop is proven.

## Trust Boundary

The safest initial trust boundary is:

MaintainerBot may propose changes. Humans approve releases.

That means the early service can still be valuable without requiring maintainers
to hand over full crate publishing authority.

Allowed early actions:

- Open dependency update PRs.
- Open security remediation PRs.
- Open CI fix PRs.
- Open documentation/example cleanup PRs.
- Triage issues with labels or comments if granted permission.
- Summarize stale PRs and likely merge blockers.
- Recommend release notes and version bumps.

Gated later actions:

- Merge PRs.
- Publish crates.io releases.
- Transfer repository permissions.
- Act on third-party contributor PRs without maintainer approval.

## HiveCore UX Shape

MaintainerBot should appear in HiveCore as an operating mode or workflow, not as
a 12th product tile.

Possible HiveCore surfaces:

- **MaintainerOps setup**: configure crate repositories, tokens, SLA goals,
  policy, and allowed action types.
- **Crate fleet map**: show each crate's maintenance status.
- **Maintenance queue**: dependency, security, CI, issue, docs, and release
  tasks grouped by urgency.
- **Evidence timeline**: show checks, scans, PRs, warnings, and human approvals.
- **Policy controls**: decide what MaintainerBot can do automatically versus
  what requires approval.
- **SLA screen**: show whether a crate is green, attention-needed, or blocked.

This makes HiveCore feel like the brain of the service.

## Suggested Workflow

1. HiveCore registers a Rust crate target.
2. SignalHive checks repo activity, issues, stale PRs, and backlog pressure.
3. DepTriage checks dependency update pressure.
4. VulnTriage checks advisories and security alerts.
5. FlakeSting checks CI instability across recent runs.
6. RepoMemory loads crate conventions and release history.
7. HiveCore creates a maintenance plan.
8. TrustGate evaluates proposed changes before write actions.
9. RepoReaper opens safe PRs where appropriate.
10. MergeKeeper and ReviewBee track whether those PRs are ready, blocked, or
    waiting on feedback.
11. HiveCore records the result as SLA evidence.

The loop should be repeatable on a schedule.

## Business Shape

The business offering should be framed around maintenance outcomes, not tool
access.

Potential positioning:

> MaintainerBot by PatchHive keeps Rust crates maintained with autonomous,
> reviewable maintenance PRs and visible health evidence.

Possible pricing experiments:

- Per crate per month.
- Volume discount for multiple crates.
- Free trial for friendly maintainers.
- Higher tier for private repos or stricter SLA reporting.

The first commercial promise should be modest:

- "We monitor and propose maintenance work."
- Not "we become your full maintainer and publish releases."

## Good First Pilot

The pilot should start with crates where the risk is controlled.

Good pilot targets:

- PatchHive-owned crates.
- Small friendly maintainer crates.
- Low-risk docs/dependency/CI maintenance.
- Repositories with clear tests.
- Repositories where maintainers explicitly opt in.

Avoid early:

- Critical ecosystem crates with huge blast radius.
- Crates requiring complex unsafe-code changes.
- Repos with unclear licensing or governance.
- Anything requiring crates.io publish permissions.

## MVP Requirements

Before calling this a real MaintainerBot MVP, PatchHive should have:

- HiveCore target configuration for Rust crate repositories.
- A Rust crate policy pack.
- Scheduled read-only scans across the target set.
- Dependency/security/CI/doc issue grouping.
- Safe RepoReaper dry-run evidence.
- TrustGate checks before write-capable PRs.
- A clear "proposed PR" path with PatchHive attribution.
- A crate health summary that can be shown to maintainers.

## Open Questions

- Should MaintainerBot be Rust-only at first, or Rust-first with a path to other
  ecosystems?
- What is the minimum permission set needed for a useful trial?
- Should issue comments be enabled early, or should v1 avoid community-facing
  comments and only open PRs?
- What counts as SLA success: time to PR, time to triage, dependency freshness,
  security response time, or a combined score?
- How should HiveCore separate "maintainer-approved automation" from "operator
  testing automation"?

## Decision

MaintainerBot should be preserved as a future operating mode and service package
for PatchHive. It should not become a standalone product directory until there
is strong evidence that it needs custom behavior that cannot be cleanly modeled
as HiveCore orchestration plus existing specialist products.

For now, build the suite. Then package the suite's Rust crate maintenance flow
as MaintainerBot.
