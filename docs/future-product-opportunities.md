# Future Product Opportunities

This document evaluates product concepts recovered from an older Lovable UI
against the current PatchHive lineup. It is a planning record, not a commitment
to create every concept as a standalone product.

The main test is whether a concept owns a distinct maintenance question, data
model, safety boundary, and workflow. If it does not, it should strengthen an
existing product or become a shared platform capability instead of adding
another product surface.

## Summary

| Original concept | Original role | Decision | Recommended home or working name |
| --- | --- | --- | --- |
| DepWarden | Dependency drift and CVE remediation | Extend existing products | DepTriage + VulnTriage + RepoReaper |
| FlakeDoctor | Flaky test triage and quarantine | Extend existing product | FlakeSting |
| CISentinel | CI failure clustering and root-cause analysis | Strong new-product candidate | BuildSentry |
| PerfTracer | Regression detection from CI traces | Strong new-product candidate | PerfSentry or BenchSting |
| DocMason | Documentation drift and changelog stewardship | Conditional new-product candidate | DocKeeper |
| SecretShade | Secret leak detection and rotation | Strong new-product candidate | SecretSentry |
| MergePilot | Merge queue and conflict resolution | Extend existing product | MergeKeeper |
| AuditMesh | Evidence vault and audit replay | Shared platform capability | HiveCore evidence vault and replay |

The clearest additions are BuildSentry, a performance-regression product, and
SecretSentry. DocKeeper also owns useful work, but its changelog responsibilities
must remain distinct from ReleaseSentry.

## Concept Evaluations

### DepWarden

Dependency drift and CVE remediation crosses three existing product boundaries:

- **DepTriage** decides which dependency updates deserve attention now.
- **VulnTriage** prioritizes dependency and code-security findings.
- **RepoReaper** is the write-capable product that can eventually implement,
  validate, and submit an approved remediation.

Dependency drift is a valuable DepTriage capability. CVE urgency belongs in
VulnTriage, while remediation should be an explicit, safety-gated handoff to
RepoReaper. Creating DepWarden would duplicate all three responsibilities and
blur the suite's read-versus-write boundary.

**Decision:** Do not create a standalone product. Add dependency-drift evidence
to DepTriage and design a typed VulnTriage/DepTriage-to-RepoReaper remediation
handoff later.

### FlakeDoctor

This concept is almost exactly FlakeSting's domain. FlakeSting already detects,
explains, and prioritizes flaky tests and unstable workflow behavior.

Quarantine is a meaningful future workflow, but it changes FlakeSting from a
read-only signal product into a product capable of suppressing or modifying CI
behavior. It should therefore be introduced behind visible evidence, explicit
approval, repository policy, expiration, and reversal controls. RepoReaper may
perform any resulting code or configuration change.

**Decision:** Do not create a standalone product. Treat triage and controlled
quarantine as later FlakeSting capabilities.

### CISentinel

General CI failure diagnosis is not the same as flake detection. FlakeSting asks
whether a test or workflow signal can be trusted. This product would explain why
a currently failing build is red, including deterministic test failures,
compilation failures, environment problems, infrastructure failures, and
recurring failure clusters.

This has a distinct input model, output queue, and suite role. Its evidence could
improve FlakeSting, MergeKeeper, ReleaseSentry, and RepoReaper.

**Decision:** Strong standalone candidate.

**Recommended working name:** **BuildSentry** — CI failure clustering,
infrastructure-failure identification, and root-cause evidence.

### PerfTracer

No current PatchHive product owns performance baselines, benchmark history, or
regression detection. The product would compare CI traces and benchmark results,
identify meaningful regressions, account for noisy measurements, and show the
change and evidence that caused an alert.

This is distinct from BuildSentry: a build can pass while becoming materially
slower. It is also distinct from ReleaseSentry, which should consume the
performance decision as release evidence instead of implementing performance
analysis itself.

**Decision:** Strong standalone candidate.

**Working-name options:**

- **PerfSentry** — clearest description and strongest connection to readiness.
- **BenchSting** — more distinctive PatchHive character, but narrower if the
  product later analyzes production or trace data beyond benchmarks.

### DocMason

Documentation drift is currently unowned. Examples include stale setup steps,
invalid configuration names, broken internal references, API examples that no
longer match code, and missing documentation for changed behavior.

Changelog and release-note stewardship partially overlaps ReleaseSentry.
ReleaseSentry should continue to decide whether release documentation is ready
and may generate release notes. A documentation product should instead own
ongoing documentation-to-code consistency and provide its findings as evidence
to ReleaseSentry.

**Decision:** Valid standalone candidate if kept centered on documentation
drift. It is lower priority than CI, performance, and secret-safety work.

**Recommended working name:** **DocKeeper** — documentation drift, stale
examples, broken references, and documentation coverage.

### SecretShade

Secret detection is related to VulnTriage but operationally distinct. A leaked
credential has a lifecycle: identify the exposure, determine scope, revoke or
rotate it, update dependent systems, verify recovery, and preserve an audit
trail. That workflow carries stronger authorization and safety requirements than
ordinary finding prioritization.

VulnTriage may ingest and prioritize secret-scanning alerts, but a specialist
product can own exposure response. Automated rotation should not be part of the
initial read-only release. Later rotation must require explicit approval,
provider-specific adapters, least-privilege credentials, redacted evidence, and
recovery or rollback guidance.

**Decision:** Strong standalone candidate, introduced detection-first and
read-only before any rotation capability.

**Recommended working name:** **SecretSentry** — leaked-secret detection,
exposure triage, and carefully gated rotation.

### MergePilot

Merge queue management and conflict resolution are natural extensions of
MergeKeeper. MergeKeeper already owns the question of whether a pull request is
ready, on hold, or blocked. Queue position, dependency ordering, conflict state,
and safe conflict-resolution handoffs deepen that decision rather than create a
new maintenance domain.

HiveCore may orchestrate work across many products, but the pull-request merge
decision should remain product-owned by MergeKeeper. Any autonomous code change
needed to resolve a conflict can be handed to RepoReaper with TrustGate and test
validation in the path.

**Decision:** Do not create a standalone product. Expand MergeKeeper with merge
queue and conflict-resolution coordination.

### AuditMesh

Evidence storage and replay are valuable across every product. Making them a
specialist product would encourage products to send operational truth into a
separate silo. Instead, products should emit standardized run events and
diagnostic artifacts; HiveCore should index and present that evidence through a
suite-wide vault and replay surface.

RepoMemory can retain durable lessons derived from outcomes, but it should not
replace immutable run evidence. Evidence replay should reproduce the inputs,
decisions, versions, policy, and artifacts associated with a run without
silently repeating its write actions.

**Decision:** Build this as a shared backend and HiveCore capability, not a
standalone specialist product. A separately packaged compliance product should
only be reconsidered if external audit workflows become a real customer need.

## Recommended Product Boundaries

The CI-related products should answer different questions:

| Product | Question it answers |
| --- | --- |
| FlakeSting | Can this test or workflow signal be trusted? |
| BuildSentry | Why is CI currently failing? |
| PerfSentry | Did this change make the product materially slower? |
| MergeKeeper | Is this pull request safe and ready to merge? |
| ReleaseSentry | Should the resulting release ship? |

The dependency and security workflow should preserve explicit handoffs:

1. DepTriage detects dependency drift and ranks routine updates.
2. VulnTriage ranks security findings and dependency vulnerabilities.
3. TrustGate and RepoMemory provide policy and repository context.
4. RepoReaper performs an explicitly authorized remediation and validates it.
5. MergeKeeper and ReleaseSentry evaluate whether the result can merge and ship.

## Suggested Priority

1. **BuildSentry** — fills the clearest gap and strengthens four existing
   products with shared CI root-cause evidence.
2. **PerfSentry** — establishes performance as a first-class release signal.
3. **SecretSentry** — adds a distinct security-response workflow, beginning
   with read-only detection and exposure triage.
4. **DocKeeper** — continuously detects documentation drift and supplies release
   evidence after the more central automation pipeline is mature.

These priorities describe product value, not immediate implementation order.
Per the UI v3 and unified-backend migration rules, the current integrated
product set should reach parity before another engine or frontend is started.

## Naming Status

BuildSentry, PerfSentry, BenchSting, SecretSentry, and DocKeeper are working
names only. Before scaffolding a product, confirm that the name:

- is distinct from current PatchHive product responsibilities;
- fits the suite's concise specialist naming style;
- has an available product slug and GitHub repository name;
- does not imply autonomous write access before the product supports it safely;
- remains accurate if the initial capability expands.

