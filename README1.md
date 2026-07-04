# PatchHive

<p align="center">
  <img src="./patchhive.png" width="120" alt="PatchHive logo" />
</p>

<p align="center">
  <strong>Autonomous software maintenance that stays visible, reviewable, and clearly attributed.</strong>
</p>

PatchHive is a software maintenance platform for finding, prioritizing, and eventually fixing maintenance work across repositories.

It is not another chat-first coding assistant. PatchHive is built around autonomous, outbound contribution: the operator chooses broad scopes and safety settings, products discover useful work, evidence stays visible, and write actions happen only through constrained, reviewable paths.

When PatchHive opens a pull request or comments on an issue, it should be obvious what happened, why it happened, and that the work came from PatchHive.

## What PatchHive Is Built To Do

PatchHive turns maintenance into an operating loop:

1. Find maintenance pressure before it becomes urgent.
2. Rank the work that actually deserves attention.
3. Check risk before automation touches code.
4. Remember repo-specific conventions and painful failures.
5. Open clearly attributed pull requests only when the evidence supports it.
6. Coordinate the suite from one control plane without making every product dependent on it.

The goal is not to make automation look human. The goal is to make automation useful enough, transparent enough, and constrained enough that maintainers can judge it on the work.

## Product Suite

| Product | Role |
| --- | --- |
| **SignalHive** | Finds stale work, duplicate issues, recurring bug patterns, TODO/FIXME hotspots, and maintenance drag. |
| **ReviewBee** | Converts PR review threads into a concrete follow-up checklist. |
| **TrustGate** | Reviews diffs against repo-specific safety rules and returns `safe`, `warn`, or `block`. |
| **RepoMemory** | Stores durable repo conventions, review feedback, hotspot history, and failure lessons. |
| **MergeKeeper** | Decides whether a PR is ready, blocked, or on hold. |
| **FlakeSting** | Detects flaky CI behavior and explains why the signal is unstable. |
| **DepTriage** | Ranks dependency update noise by urgency and practical impact. |
| **VulnTriage** | Turns security alerts into an engineering queue with clear next steps. |
| **RefactorScout** | Surfaces conservative, high-value refactor opportunities. |
| **ReleaseSentry** | Checks release readiness from CI, tags, changelog, blocker issues, and release evidence. |
| **RepoReaper** | Finds fixable issues, generates patches, validates them, and opens attributed pull requests. |
| **HiveCore** | The suite cockpit for health, launch control, shared defaults, run history, and product handoffs. |

## How The Pieces Fit

| Layer | Products | Purpose |
| --- | --- | --- |
| Discovery and signals | SignalHive, ReviewBee, FlakeSting, DepTriage, VulnTriage, RefactorScout, ReleaseSentry | Surface maintenance pressure from issues, PRs, CI, dependencies, security alerts, release state, and code structure. |
| Trust and memory | TrustGate, RepoMemory, MergeKeeper, FailGuard | Evaluate risk, preserve repo-specific lessons, convert bad outcomes into guardrails, and decide when PRs are truly ready. |
| Autonomous action | RepoReaper | Turns trusted candidate work into validated patches and clearly attributed pull requests. |
| Control plane | HiveCore | Makes the suite legible in one place and coordinates shared defaults, status, history, and handoffs. |

PatchHive matures automation in that order: visibility first, trust and memory second, autonomous write actions after the foundation exists.

## Attribution And Trust

PatchHive contributions should be direct about their origin.

- Autonomous PRs come from the PatchHive GitHub identity.
- PR bodies and issue comments identify the product that acted.
- Evidence, confidence, validation posture, and failure reasons should stay visible.
- Maintainers should be able to inspect PatchHive's history and decide whether the work is worth trusting.

Transparency is part of the product, not a disclaimer bolted on afterward.

## Current Status

PatchHive is in active alpha and is being built for real operator use first. The suite already includes product backends, v2 frontend surfaces, shared Rust crates, shared UI packages, standalone product mirrors, and a unified backend migration in progress.

Current engineering focus:

- Move product engines into the shared PatchHive backend one at a time.
- Keep every product independently runnable while the suite converges.
- Harden HiveCore into the control plane for status, shared settings, schedules, and product handoffs.
- Continue testing real read-only scans and guarded write actions before expanding autonomy.

## Repository Model

PatchHive is developed monorepo-first. Products and shared foundations are built together, then exported to standalone mirrors under this organization so each product can also be inspected and run on its own.

The monorepo remains the source of truth. Standalone repos are product-facing mirrors.

## Core Principles

- Maintenance work should be continuously visible.
- Automation should be constrained and reviewable.
- Repo-specific memory should improve future decisions.
- Outbound contribution should be clearly attributed.
- Safety gates should come before autonomous write actions.
- Trust should be earned through signal quality and consistent history.

## For Maintainers

If PatchHive opens a PR or comments on an issue in your repository, treat it like any other automated contribution:

- Read the linked issue, PR body, and generated explanation.
- Check whether tests were run or whether the PR is intentionally draft.
- Review the diff like you would review work from any external contributor.
- Close or request changes if the work is not useful.

PatchHive is designed to make that review possible without pretending the work came from a human.
